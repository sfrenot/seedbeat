package main

import "time"
import "fmt"
import "net"
import "encoding/binary"
import "sync"
import "sync/atomic"
import "os"
import "runtime"
import "io"
import "bufio"
import "strings"
import "flag"
import "bc/bcmessage"


const NBGOROUTINES = 800

var connectionStartChannel chan string

var beatOn bool

var peerLogFile *os.File
var addressChannel chan string

var addressesToTest int32
var startTime = time.Now()

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

var addressesVisited = make(map[string]*peerStatus)
var addressesStatusMutex sync.Mutex

func isWaiting(aPeer string) bool {
  addressesStatusMutex.Lock()
  peer, found := addressesVisited[aPeer]
  isWaiting := false
  if !found {
    addressesVisited[aPeer] = &peerStatus{Connecting, 0}
    isWaiting = true
  } else if peer.status == Waiting {
    addressesVisited[aPeer].status = Connecting
    isWaiting = true
  }
  addressesStatusMutex.Unlock()
  return isWaiting
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

func parseArgs() string{
  var outputFileName = flag.String("o", "", "Fichier de sortie du crawling")
  var initialAddress = flag.String("s", "", "Adresse initiale de crawling. Format [a.b.c.d]:ppp")
  flag.BoolVar(&beatOn, "b", false, "Flag pour le mode beat")
  flag.Parse()
  if *initialAddress == "" {
    flag.PrintDefaults()
    os.Exit(1)
  }
  if *outputFileName != "" {
    peerLogFile, _ = os.Create(*outputFileName)
  }
  return *initialAddress
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
func processAddrMessage(targetAddress string, payload []byte) int {
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
        fmt.Println("POOL Error ", readAddr, payload)
      }
      timefield:=payload[addrBeginsat:addrBeginsat+4]
      timeint := int64(binary.LittleEndian.Uint32(timefield[:]))
      timetime := time.Unix(timeint,0)
      services:=payload[addrBeginsat+4:addrBeginsat+12]
      ipAddr := payload[addrBeginsat+12 : addrBeginsat+28]
      port := payload[addrBeginsat+28 : addrBeginsat+30]
      // fmt.Println("Received : ", net.IP.String(ipAddr))
      newPeer := fmt.Sprintf("[%s]:%d", net.IP.String(ipAddr), binary.BigEndian.Uint16(port))
      peerrecordstring := fmt.Sprintf("PAR\t[%s]:%d\t%v\t%v\t%v\t%v\t%v\n", net.IP.String(ipAddr), binary.BigEndian.Uint16(port), timetime.String(), time.Since(timetime), time.Now().String(), services, targetAddress)
      storeEvent(peerrecordstring)
      // io.Copy(peerLogFile, strings.NewReader(peerrecordstring))
      addressChannel <- newPeer
      readAddr++
    }
  }
  return int(addrNumber)
}

func eightByteLittleEndianTimestampToTime(buf []byte) time.Time {
  timeint := int64(binary.LittleEndian.Uint64(buf[:]))
  return time.Unix(timeint,0)
}

func processVersionMessage(peerID string, payload []byte){

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
  storeEvent(fmt.Sprintf("PVM\t%s\t%v\t%s\t%v\t%v\t%v\t%v\n",peerID,versionNumber,useragentString, peertimestamp.String(), time.Since(peertimestamp), servicesbuf[:],time.Now().String()))
  // io.Copy(peerLogFile, strings.NewReader(fmt.Sprintf("PVM  %s  %v  %s  %v  %v  %v  %v\n",peerID,versionNumber,useragentString, peertimestamp.String(), time.Since(peertimestamp), servicesbuf[:],time.Now().String())))
  registerPVMConnection(peerID)
}

func handleIncommingMessages(targetAddress string, inChan chan []string, rawConn net.Conn) {
  rawConn.SetReadDeadline(time.Now().Add(1*time.Minute))
  connE := bufio.NewReader(rawConn)

  for {
    command, payload, err := bcmessage.ReadMessage(connE)
    if err !=  nil {
      inChan <- []string{"CONNCLOSED"}
      break
    }

    if command == bcmessage.MSG_VERSION && len(payload) > 0 {
      processVersionMessage(targetAddress, payload)
      inChan <- []string{bcmessage.MSG_VERSION}
      continue
    }
    if command == bcmessage.MSG_VERSION_ACK {
      inChan <- []string{bcmessage.MSG_VERSION_ACK}
      continue
    }
    if command == bcmessage.MSG_ADDR {
      numAddr := processAddrMessage(targetAddress, payload)
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

func checkPoolSizes(){
  for{
    time.Sleep(1* time.Duration(time.Second))
    fmt.Printf("POOLSIZE ADDR %d GOROUTINES %d\n", addressesToTest, runtime.NumGoroutine())
    if (addressesToTest == 0){
        fmt.Println("POOL Crawling ends : ", time.Now().Sub(startTime))
        os.Exit(0)
    }
  }
}

func handleOnePeer(agentNumber int, connectionStartChannel chan string) {
  for {
    // fmt.Println("POOL reading agent ", agentNumber)
    targetaddress := <- connectionStartChannel
    // fmt.Println("POOL unlock agent ", agentNumber, targetaddress)

    // fmt.Println("Targeting |" + targetaddress + "|")
    conn, err := net.DialTimeout("tcp", targetaddress, time.Duration(600*time.Millisecond))
    if err != nil {
      // fmt.Println("Failed on connect " + targetaddress)
      retryAddress(targetaddress)
      // fmt.Println("POOL Failed on connect " + targetaddress + " " + err.Error())
      // io.Copy(peerLogFile, strings.NewReader(fmt.Sprintf("ERR %s\n", targetaddress)))
    } else {

      for {
        // fmt.Println("Connected to " + targetaddress)
        //SFR Pareil ?
        // io.Copy(peerLogFile, strings.NewReader(fmt.Sprintf("Connected to %s\n", targetaddress)))
        inChan := make(chan []string, 10)
        go handleIncommingMessages(targetaddress, inChan, conn)

        err := bcmessage.SendRequest(conn, bcmessage.MSG_VERSION)
        if err != nil {
          fail(targetaddress)
          break
        }
        rcvdMessage := <-inChan
        if rcvdMessage[0] != bcmessage.MSG_VERSION {
          // fmt.Println("Version Ack not received", rcvdMessage[0])
          fail(targetaddress)
          break
        }

        err = bcmessage.SendRequest(conn, bcmessage.MSG_VERSION_ACK)
        if err != nil {
          fail(targetaddress)
          break
        }
        rcvdMessage = <-inChan
        if rcvdMessage[0] != bcmessage.MSG_VERSION_ACK {
          // fmt.Println("Version AckAck not received")
          fail(targetaddress)
          break
        }

        err = bcmessage.SendRequest(conn, bcmessage.MSG_GETADDR)
        if err != nil {
          fail(targetaddress)
          break
        }
        rcvdMessage = <-inChan
        if rcvdMessage[0] == "CONNCLOSED" {
          done(targetaddress)
          break
        } else {
          fmt.Println("Bad message", rcvdMessage[0])
          os.Exit(1)
        }
      } //For Loop that handles broken bcmessage
      conn.Close()
    }
    atomic.AddInt32(&addressesToTest, -1)
  }
}

func storeEvent(msg string) {
  if peerLogFile != nil {
    // fmt.Println("fichier")
    io.Copy(peerLogFile, strings.NewReader(msg))
  }
  if beatOn {
    // fmt.Println("beat")
  }
}

func main() {

  addressChannel = make(chan string, 1000000)
  connectionStartChannel = make(chan string, 1000000)

  addressChannel <- parseArgs()
  defer cleanAll()

  go checkPoolSizes()

  for i := 0; i < NBGOROUTINES; i++ {
    go handleOnePeer(i, connectionStartChannel)
  }

  for {
    // time.Sleep(time.Duration((1) * time.Millisecond))
    //fmt.Printf("MAIN: NUMGOROUTINE : %v\n",runtime.NumGoroutine())
    //fmt.Printf("Checking for new peer\n")
    newPeer := <-addressChannel
    if isWaiting(newPeer) { //Peer Inconnu
      atomic.AddInt32(&addressesToTest, 1)
      connectionStartChannel <- newPeer
      //fmt.Printf("Peer %s to be contacted, known peers : %d\n", newPeer, len(addressesVisited))
      //SFR Mode info ?
      // io.Copy(peerLogFile, strings.NewReader(fmt.Sprintf("NEW Address received %s\n",newPeer )))
    }
  }
}

func cleanAll() {
  if peerLogFile != nil {
    peerLogFile.Close()
  }
}
