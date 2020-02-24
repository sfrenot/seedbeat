package bctools

import (
	"time"
	"strings"
	"net"

  // "os"
	// "log"
	// "strconv"
	"github.com/elastic/beats/libbeat/logp"
  "os/exec"
)

type PeerTestStruct struct {
	Peer string
	IsNew bool
	Status bool
}

type DiggedSeedStruct struct {
	Crypto string
	Seed string
	Peers []string
}

func ParseSeeds(crypto string, seed string, peerResChan chan <- DiggedSeedStruct) {
	peerRes := make([]string, 0)
	// logp.Info("dig " + seed)
	out, err := exec.Command("dig", seed).CombinedOutput()
	if err != nil {
		// exit status 9
		logp.Info(err.Error())
	} else {
		// logp.Info("digged " + seed)

		digString := string(out)
		digLines := strings.Split(digString, "\n")
		//fmt.Println(len(digLines))

		cpt := 0
		for _, line := range digLines {
			cpt = cpt + 1
			if line == ";; ANSWER SECTION:" {
				break
			}
		}
		for i := cpt; i < len(digLines); i++ {
			line := digLines[i]
			if line == "" {
				break
			}

			// fmt.Println("LINE:" + line)
			record := strings.Split(line, "\t")
			address := record[len(record)-1]

			// typeDNS := record[len(record)-2]
			//fmt.Println("Stitching " + address+ "-")
			// fmt.Println("Type " + typeDNS+ "-")

			if record[len(record)-2] == "A" && address != "" {
				peerRes = append(peerRes, address)
		  }
		}
	}
	peerResChan <-DiggedSeedStruct{crypto, seed, peerRes}
}

func PeerTester(peer string, port string, isNew bool, peerTest chan <- PeerTestStruct){
  conn, err := net.DialTimeout("tcp", peer+":"+port, time.Duration(1*time.Second))
	res:=true
  if err != nil {
		res=false
	} else {
	 	conn.Close()
	}
	peerTest <-PeerTestStruct{peer, isNew, res}
}
