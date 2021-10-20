package beater

import (
  "time"
  "fmt"
  "net"
  "encoding/binary"
  "sync"
  "sync/atomic"
  "os"
  "runtime"
  "strings"
  // "io"
  "bufio"
  "math/rand"

  "github.com/elastic/beats/libbeat/beat"
  "github.com/elastic/beats/libbeat/common"
  "github.com/elastic/beats/libbeat/logp"

  "github.com/sfrenot/seedbeat/bccrawler/config"

  "github.com/sfrenot/seedbeat/bccrawler/beater/bc/bcmessage"
  "github.com/sfrenot/seedbeat/bccrawler/beater/bctools"

)

const DONE = "Done"

type BcExplorer struct {
	done   chan struct{}
	config config.Config
	client beat.Client
}

var connectionStartChannel chan string = make(chan string, 1000000)
var addressChannel chan string = make(chan string, 1000000)

var addressesToTest int32
var startTime = time.Now()
var runNumber int32

// Peer Status Management
type status int
const (
  Waiting status = iota
  Connecting
  Connected
  Done
  Failed
)

func (s status) String() string {
  return [...]string{"Waiting", "Connecting", "Connected", "Done", "Failed"}[s]
}

type peerStatus struct {
  status status
  retries int
}

var addressesVisited map[string]*peerStatus
var addressesStatusMutex sync.Mutex
var crawlHappened bool // Indicates whether a trial found peers

func getInfo(aPeer string) (bool, bool) {
  //logp.Info("Testing peer, %v", aPeer)
  addressesStatusMutex.Lock()
  peer, found := addressesVisited[aPeer]
  isWaiting := false
  isNew := false
  if !found {
    addressesVisited[aPeer] = &peerStatus{Connecting, 0}
    isWaiting = true
    isNew = true
  } else if peer.status == Waiting {
    addressesVisited[aPeer].status = Connecting
    isWaiting = true
  }
  addressesStatusMutex.Unlock()
  //logp.Info("Waiting %v", isWaiting)
  return isNew, isWaiting
}

func registerPVMConnection(aPeer string) {
  addressesStatusMutex.Lock()
  addressesVisited[aPeer].status = Connected
  addressesStatusMutex.Unlock()
}

func retryAddress(aPeer string) {
  addressesStatusMutex.Lock()
  if addressesVisited[aPeer].retries > 3 {
    addressesVisited[aPeer].status = Failed
  } else {
    addressesVisited[aPeer].status = Waiting
  }
  addressesStatusMutex.Unlock()
}
func fail(aPeer string){
  addressesStatusMutex.Lock()
  addressesVisited[aPeer].status = Failed
  addressesStatusMutex.Unlock()
}

func done(aPeer string) {
  addressesStatusMutex.Lock()
  addressesVisited[aPeer].status = Done
  addressesStatusMutex.Unlock()
}

func getcompactIntAsInt(bytes []byte) uint64 {
  if bytes[0] == 0xFD {
    return uint64(binary.LittleEndian.Uint16(bytes[1:3]))
  } else {
    if bytes[0] == 0xFE {
      return uint64(binary.LittleEndian.Uint32(bytes[1:5]))
    } else {
      if bytes[0] == 0xFF {
        return uint64(binary.LittleEndian.Uint64(bytes[1:9]))
      } else {
        return uint64(uint8(bytes[0]))
      }
    }
  }
}

// In Message Mgmt
func processAddrMessage(bt *BcExplorer, targetAddress string, payload []byte) int {
  if len(payload) == 0 {return 0}

  addrNumber := getcompactIntAsInt(payload)
  if addrNumber > 1 {
    startByte := uint64(0)
    if addrNumber < 253 {
      startByte = 1
    } else {
      if addrNumber < 0xFFFF {
        startByte = 3
      } else {
        if addrNumber < 0xFFFFFFFF {
          startByte = 5
        } else {
          startByte = 9
        }
      }
    }

    // fmt.Printf("Received %d addresses\n", addrNumber)
    readAddr := uint64(0)
    for {
      if readAddr == addrNumber {
        break
      }
      addrBeginsat := startByte + (30 * readAddr)
      if (addrBeginsat+4) > uint64(len(payload)) {
        logp.Info("POOL Error %v %v ", readAddr, payload)
      }
      timefield:=payload[addrBeginsat:addrBeginsat+4]
      timeint := int64(binary.LittleEndian.Uint32(timefield[:]))
      timetime := time.Unix(timeint,0)
      services:=payload[addrBeginsat+4:addrBeginsat+12]
      ipAddr := payload[addrBeginsat+12 : addrBeginsat+28]
      port := payload[addrBeginsat+28 : addrBeginsat+30]
      // fmt.Println("Received : ", net.IP.String(ipAddr))
      newPeer := fmt.Sprintf("[%s]:%d", net.IP.String(ipAddr), binary.BigEndian.Uint16(port))

      //logp.Info("coucou -> %v", newPeer)
      new, waiting := getInfo(newPeer)
      if bt.config.ParEvent && new { bt.emitEvent("PAR", net.IP.String(ipAddr), 0, "", services, timetime, targetAddress[1:strings.Index(targetAddress, "]")])}
      if waiting { addressChannel <- newPeer }

      readAddr++
    }
  }
  return int(addrNumber)
}

func eightByteLittleEndianTimestampToTime(buf []byte) time.Time {
  timeint := int64(binary.LittleEndian.Uint64(buf[:]))
  return time.Unix(timeint,0)
}

func processVersionMessage(bt *BcExplorer, pvmID string, payload []byte){

  versionNumber := binary.LittleEndian.Uint32(payload[0:4])
  servicesbuf := payload[4:12] //services
  peertimestamp := eightByteLittleEndianTimestampToTime(payload[12:20])

  userAgentStringSize := int(getcompactIntAsInt(payload[80:]))
  startByte :=0
  useragentString := ""
  if userAgentStringSize < 253 {
    startByte = 1
  } else {
    if userAgentStringSize < 0xFFFF {
      startByte = 3
    } else {
      if userAgentStringSize < 0xFFFFFFFF {
        startByte = 5
      } else {
        startByte = 9
      }
    }
  }
  if 80+startByte+userAgentStringSize < len(payload) {
    if userAgentStringSize > 0{
      useragentbuf := payload[80+startByte:80+startByte+userAgentStringSize]
      useragentString = string(useragentbuf)
    }
  }

  bt.emitEvent("PVM", "127.0.0.1", versionNumber, useragentString, servicesbuf, peertimestamp, pvmID[1:strings.Index(pvmID, "]")])
  getInfo(pvmID)
  registerPVMConnection(pvmID)
}

func handleIncommingMessages(bt *BcExplorer, targetAddress string, inChan chan []string, rawConn net.Conn) {
  rawConn.SetReadDeadline(time.Now().Add(1*time.Minute))
  connE := bufio.NewReader(rawConn)

  for {
    command, payload, err := bcmessage.ReadMessage(connE)
    if err !=  nil {
      inChan <- []string{"CONNCLOSED"}
      break
    }

    if command == bcmessage.MSG_VERSION && len(payload) > 0 {
      processVersionMessage(bt, targetAddress, payload)
      inChan <- []string{bcmessage.MSG_VERSION}
      continue
    }
    if command == bcmessage.MSG_VERSION_ACK {
      inChan <- []string{bcmessage.MSG_VERSION_ACK}
      continue
    }
    if command == bcmessage.MSG_ADDR {
      numAddr := processAddrMessage(bt, targetAddress, payload)
      if numAddr > 5 {
        inChan <- []string{"CONNCLOSED"}
        break
      }
      continue
    }
    // fmt.Println("->", command)
    continue
  }
}

//TODO: handleOneMessage function
func handleOnePeer(bt *BcExplorer, agentNumber int) {
  for {
    // fmt.Println("POOL reading agent ", agentNumber)
    targetAddress := <- connectionStartChannel
    if targetAddress == "quit" {return}
    //logp.Info("POOL unlock agent %v %v ", agentNumber, targetAddress)

    // fmt.Println("Targeting |" + targetAddress + "|")
    conn, err := net.DialTimeout("tcp", targetAddress, time.Duration(600*time.Millisecond))
    // fmt.Println("conn", conn)
    if err != nil {
      // fmt.Println("Failed on connect " + targetAddress)
      retryAddress(targetAddress)
      // fmt.Println("POOL Failed on connect " + targetAddress + " " + err.Error())
      // io.Copy(peerLogFile, strings.NewReader(fmt.Sprintf("ERR %s\n", targetAddress)))
    } else {

      for {
        // fmt.Println("Connected to " + targetAddress)
        //SFR Pareil ?
        // io.Copy(peerLogFile, strings.NewReader(fmt.Sprintf("Connected to %s\n", targetAddress)))
        inChan := make(chan []string, 10)
        go handleIncommingMessages(bt, targetAddress, inChan, conn)

        err := bcmessage.SendRequest(conn, bcmessage.MSG_VERSION)
        if err != nil {
          fail(targetAddress)
          break
        }
        rcvdMessage := <-inChan
        if rcvdMessage[0] != bcmessage.MSG_VERSION {
          // fmt.Println("Version Ack not received", rcvdMessage[0])
          fail(targetAddress)
          break
        }

        err = bcmessage.SendRequest(conn, bcmessage.MSG_VERSION_ACK)
        if err != nil {
          fail(targetAddress)
          break
        }
        rcvdMessage = <-inChan
        if rcvdMessage[0] != bcmessage.MSG_VERSION_ACK {
          // fmt.Println("Version AckAck not received")
          fail(targetAddress)
          break
        }

        err = bcmessage.SendRequest(conn, bcmessage.MSG_GETADDR)
        if err != nil {
          fail(targetAddress)
          break
        }
        rcvdMessage = <-inChan
        if rcvdMessage[0] == "CONNCLOSED" {
          done(targetAddress)
          break
        } else {
          logp.Info("Bad message %v", rcvdMessage[0])
          os.Exit(1)
        }
      } //For Loop that handles broken bcmessage
      conn.Close()
    }
    atomic.AddInt32(&addressesToTest, -1)
  }
}

func (bt *BcExplorer) emitEvent(kind string, peerID string, version uint32, agent string, services []byte, srcTime time.Time, diggedPeer string) {
       //fmt.Println("->", runNumber)
		event := beat.Event{
			Timestamp: time.Now(),
			Fields: common.MapStr{
        "run": runNumber,
				"message": kind, // PVM or PAR
        "peer": peerID,
        "PVMversion": fmt.Sprint(version),
        "PVMagent": agent,
        "services": services,
        "srcTime": srcTime,
        "PVMPeer": diggedPeer,
        "Crypto": bt.config.Cryptos[bt.config.ObservedCrypto].Code,
      },
		}
		bt.client.Publish(event)
}

func (bt *BcExplorer) checkPoolSizes(){
  for{
    time.Sleep(bt.config.CheckForEndTimer)
    logp.Info("POOLSIZE ADDR %d GOROUTINES %d", addressesToTest, runtime.NumGoroutine())
    if !crawlHappened && addressesToTest > 5 {
      crawlHappened = true
    }
    if (addressesToTest == 0){
        logp.Info("POOL Crawling ends : %v", time.Now().Sub(startTime))
        addressChannel<-DONE
        return
    }
  }
}

// Beat
func New(b *beat.Beat, cfg *common.Config) (beat.Beater, error) {
  // fmt.Println("->", runtime.NumGoroutine())

	c := config.DefaultConfig
	if err := cfg.Unpack(&c); err != nil {
		return nil, fmt.Errorf("Error reading config file: %v", err)
	}
	bt := &BcExplorer{
		done:   make(chan struct{}),
		config: c,
	}

  runNumber = bt.config.InitialRunNumber

  for i := 0; i < bt.config.NbGoRoutines; i++ {
    go handleOnePeer(bt, i)
  }

  var err error
  bt.client, err = b.Publisher.Connect()
  if err != nil {
    return nil, fmt.Errorf("Erreur connexion au publisher : %v", err)
  }

  bcmessage.SetMagic(bt.config.Cryptos[bt.config.ObservedCrypto].Magic)

	return bt, nil
}

func (bt *BcExplorer) Run(b *beat.Beat) error {

  logp.Info("bcExplorer is running! Hit CTRL-C to stop it.")

  for {
    crawlHappened = false
    addressesVisited = make(map[string]*peerStatus)

    // Récupération d'une adresse d'initialisation
    digSrc := bt.config.Cryptos[bt.config.ObservedCrypto].Seeds[rand.Intn(len(bt.config.Cryptos[bt.config.ObservedCrypto].Seeds))]
    peersChan := make(chan bctools.DiggedSeedStruct)
    go bctools.ParseSeeds(bt.config.Cryptos[bt.config.ObservedCrypto].Code, digSrc , peersChan)
    digResponse := <-peersChan

    // ex: [134.14.143.12]:8333
    if (len(digResponse.Peers)) > 0 {
      logp.Info("Start Loop with %v, %d routines, run #%v", digSrc, runtime.NumGoroutine(), runNumber)
      bt.getPeers(fmt.Sprintf("[%s]:%s", digResponse.Peers[rand.Intn(len(digResponse.Peers))], bt.config.Cryptos[bt.config.ObservedCrypto].Port))
      wait, _ := time.ParseDuration("2s")
      if crawlHappened { wait = bt.config.Period }
      logp.Info("POOL Sleeping for %v", wait)
      ticker := time.NewTicker(wait)
      select {
        case <-bt.done:
          return nil
        case <-ticker.C:
          runNumber++
        }
    }
  }
}

func (bt *BcExplorer) getPeers(startDig string) {
  // fmt.Println("->", startDig)

  getInfo(startDig)
  addressChannel <- startDig

  go bt.checkPoolSizes()

  for {
    newPeer := <- addressChannel
    if newPeer == DONE { // Finished crawled adress
      // fmt.Println("getPeers::Done")
      logp.Info("POOL getPeers :: end")
      return
    }
    atomic.AddInt32(&addressesToTest, 1)
    connectionStartChannel <- newPeer
  }
}

// Stop stops seedbeat.
func (bt *BcExplorer) Stop() {
  // logp.Info("Arrêt avant, il reste %d routine", runtime.NumGoroutine())
  addressChannel<-DONE
  for i := 0; i < bt.config.NbGoRoutines; i++ {
    connectionStartChannel <- "quit"
  }

  bt.client.Close()
	close(bt.done)
}
