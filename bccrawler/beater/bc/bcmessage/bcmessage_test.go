package bcmessage

import "testing"
// import "fmt"
import "bytes"

//CF : Readme.messag.bc

func TestBuildRequest(t *testing.T) {
  firstPartBeforeCheckSum := []byte{249,190,180,217,118,101,114,115,105,111,110,0,0,0,0,0,105,0,0,0} // Before Checksu
  res := BuildRequest(MSG_VERSION)
  if bytes.Compare(res[0:START_CHKSUM], firstPartBeforeCheckSum) != 0 {
    t.Error("Test Firt-Part échoué : {} attendu, recu {}", firstPartBeforeCheckSum, res[0:START_CHKSUM])
  }

  secondPartBetweeChecksumAndPayloadDate := []byte{127,17,1,0,13,4,0,0,0,0,0,0}
  if bytes.Compare(res[END_CHKSUM:END_CHKSUM+DATE_OFFSET], secondPartBetweeChecksumAndPayloadDate) != 0 {
    t.Error("Test Firt-Part échoué : {} attendu, recu {}", secondPartBetweeChecksumAndPayloadDate, res[END_CHKSUM:END_CHKSUM+DATE_OFFSET])
  }

  lastPartAfterPayloadDate := []byte{0,0,0,0,0,0,0,0,13,4,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,255,255,127,0,0,1,141,32,13,4,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,255,255,127,0,0,1,141,32,20,20,20,20,20,20,20,18,12,47,98,99,112,99,58,48,46,48,46,49,47,163,218,8,0}
  if bytes.Compare(res[END_CHKSUM+DATE_OFFSET+DATE_LENGTH:], lastPartAfterPayloadDate) != 0 {
    t.Error("Test Firt-Part échoué : {} attendu, recu {}", lastPartAfterPayloadDate, res[END_CHKSUM+DATE_OFFSET+DATE_LENGTH:])
  }

  // fmt.Println("Expected ", res)
}
