package bcmessage
import "encoding/binary"
import "fmt"
import "time"
import "net"
import "strings"
import "crypto/sha256"
import "io"
// import "os"

var magic = []byte{0xF9, 0xBE, 0xB4, 0xD9}

var templateVersionMessagePayLoad [105] byte

const HEADERSIZE = 24
type header struct {
  headerBytes [HEADERSIZE] byte
}

// HEADER STRUCT
const START_MAGIC = 0
// const END_MAGIC = 4
const START_CMD = 4
const END_CMD = 16
const START_PAYLOADLENGTH = 16
const END_PAYLOADLENGTH = 20
const START_CHKSUM = 20
const END_CHKSUM = 24
const PAYLOADLENGTH_SIZE = END_PAYLOADLENGTH - START_PAYLOADLENGTH

// PAYLOAD STRUCT
const DATE_OFFSET = 12
const DATE_LENGTH = 8

const MSG_VERSION = "version"
const MSG_VERSION_ACK = "verack"
const MSG_GETADDR = "getaddr"
const MSG_ADDR = "addr"

func init() {
  var versionNumberBytes = []byte{0x7F, 0x11, 0x01, 0x00} // 127, 17, 1, 0 -> 70015 : binary.LittleEndian.PutUint32(versionNumberBytes, uint32(versionNumber))
  var servicesbuf = []byte{13,4,0,0,0,0,0,0}
  var addrrRecvSvcbuf = make([]byte, 8)
  var v6v4prefixbuf = []byte{0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF}
  var emptyV4addr = []byte{127, 0, 0, 1, 141, 32}
  var emptyDateByte = make([]byte, 8)
  var emptyV6v4NetworkAddress = append(v6v4prefixbuf, emptyV4addr...)
  var nodeID = []byte{0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x12}
  var userAgent = "\x0C/bcpc:0.0.1/"
  var height = []byte{0xA3, 0xDA, 0x08, 0x00} // 580259 : binary.LittleEndian.PutUint32(heightbuf, uint32(heightb))

  res := append(versionNumberBytes, servicesbuf...)
  res = append(res, emptyDateByte...)
  res = append(res, addrrRecvSvcbuf...)
  res = append(res, servicesbuf...)
  res = append(res, emptyV6v4NetworkAddress...)
  res = append(res, servicesbuf...)
  res = append(res, emptyV6v4NetworkAddress...)
  res = append(res, nodeID...)
  res = append(res, userAgent...)
  res = append(res, height...)

  for i, val := range res {
    templateVersionMessagePayLoad[i] = val
  }

}

// TOP Functions :
//  SendRequest to a peer returns err
//  ReadMessage from a Peer return command, payload, err
func SendRequest(conn net.Conn, messageName string) error {
  // fmt.Println("Sending ", messageName)
  _, err := conn.Write(BuildRequest(messageName))
  if err != nil {
    fmt.Println("Error on sending",  messageName, err.Error())
  }
  return err
}

func ReadMessage(conn io.Reader) (string, []byte, error)  {
  var payload []byte

  headerBuf := make([]byte, HEADERSIZE)
  _, err := io.ReadFull(conn, headerBuf)
  if err != nil {
    // fmt.Println("Erreur lecture header " + err.Error())
    return "", []byte{}, err
  }

  // checkMagic() // if err !=  nil || magicBuf[0] != magic[0] || magicBuf[1] != magic[1] || magicBuf[2] != magic[2] || magicBuf[3] != magic[3] {
  command := strings.TrimRight(string(headerBuf[START_CMD:END_CMD]), "\x00")
  payloadSize := binary.LittleEndian.Uint32(headerBuf[START_PAYLOADLENGTH:END_PAYLOADLENGTH])
  // checkChecksum(headerBuf[])...

  // fmt.Printf("Command %v PAYLOADSIZE %d\n",  command, payloadSize)

  if payloadSize > 0 {
    payload = make([]byte, payloadSize) // Lecture payload
    _, err := io.ReadFull(conn, payload)
    if err != nil {
        fmt.Println("  Erreur lecture payload " + err.Error())
        return "", []byte{}, err
    }
  }
  return command, payload, err
}

// SENDING
func BuildRequest(messageName string) [] byte {
  // fmt.Println(messageName)
  payloadBytes := []byte{}
  if messageName == MSG_VERSION {
    payloadBytes = getPayloadWithCurrentDate()
  }
  myHeader := &header{}
  buildRequestMessageHeader(myHeader, messageName, payloadBytes)
  return append(myHeader.headerBytes[:], payloadBytes...)
}

func getPayloadWithCurrentDate() [] byte {
  res := templateVersionMessagePayLoad

  date := make([]byte, DATE_LENGTH)
  binary.LittleEndian.PutUint64(date, uint64(time.Now().Unix()))
  for i, val := range date {
    res[DATE_OFFSET + i] = val
  }
  return res[:]
}

func buildRequestMessageHeader(cmBuf * header, name string, payload []byte) {
  cmBuf.injectElems(magic, START_MAGIC)
  cmBuf.injectElems([]byte(name), START_CMD)
  payloadBuf := make([]byte, PAYLOADLENGTH_SIZE)
  binary.LittleEndian.PutUint32(payloadBuf, uint32(len(payload)))
  cmBuf.injectElems(payloadBuf, START_PAYLOADLENGTH)
  cmBuf.injectElems(computeChecksum(payload), START_CHKSUM)
}

func (cmBuf * header) injectElems(src [] byte, startByte int){
  for i, val := range src {
    cmBuf.headerBytes[i+startByte] = val
  }
}

func computeChecksum(payload []byte) []byte {
  sum := sha256.Sum256(payload)
  sum = sha256.Sum256(sum[:])
  return sum[0:4]
}
