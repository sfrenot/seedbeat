package beater

import (
	"fmt"
	"time"
	"strings"
	"net"

  // "os"
	// "log"
	// "strconv"

	"github.com/elastic/beats/libbeat/beat"
	"github.com/elastic/beats/libbeat/common"
	"github.com/elastic/beats/libbeat/logp"
  "os/exec"

	"github.com/sfrenot/seedbeat/seedalltestip/config"
)

// Seedallbeat configuration.
type Seedallbeat struct {
	done   chan struct{}
	config config.Config
	client beat.Client
}

type peerTestStruct struct {
	peer string
	status bool
}

// New creates an instance of seedbeat.
func New(b *beat.Beat, cfg *common.Config) (beat.Beater, error) {
	c := config.DefaultConfig
	if err := cfg.Unpack(&c); err != nil {
		return nil, fmt.Errorf("Error reading config file: %v", err)
	}
	bt := &Seedallbeat{
		done:   make(chan struct{}),
		config: c,
	}
	return bt, nil
}

// Run starts seedbeat.
func (bt *Seedallbeat) Run(b *beat.Beat) error {
	logp.Info("seedbeat is running! Hit CTRL-C to stop it.")
	var err error

  // fmt.Println("%v", bt.config.Period)
	// os.Exit(0)

	// for _, crypto := range bt.config.Cryptos {
	// 	logp.Info(crypto.Code +" -> "+ strings.Join(crypto.Seeds, ","))
	// }
  // os.Exit(0)

	bt.client, err = b.Publisher.Connect()
	if err != nil {
		return err
	}
	ticker := time.NewTicker(bt.config.Period)

	ongoingPeers := make(map[string]map[string]map[string]bool)
	total := make(map[string]map[string]int)

	for _, crypto := range bt.config.Cryptos {    // Initialisation des structures
		// fmt.Println("->%v", crypto)
    ongoingPeers[crypto.Code] = make(map[string]map[string]bool)
		total[crypto.Code] = make(map[string]int)

	  for i := 0; i < len(crypto.Seeds); i++ {
			ongoingPeers[crypto.Code][crypto.Seeds[i]] = make(map[string]bool)
			total[crypto.Code][crypto.Seeds[i]] = 0
		}

		ongoingPeers[crypto.Code]["all"] = make(map[string]bool)
		total[crypto.Code]["all"] = 0
	}

  peerTest := make(chan peerTestStruct)
  // os.Exit(1)
	for {

		peersChan := make(chan []string) //TODO: On devrait pouvoir le sortir de la boucle
    for _, crypto := range bt.config.Cryptos { // Pour toutes les cryptos observées

	    for i := 0; i < len(crypto.Seeds); i++ {
				// logp.Info("chan -> " + strconv.Itoa(j*10+i))
				go parseSeeds(peersChan, crypto.Code, crypto.Seeds[i])
			}
		}

		for _, crypto := range bt.config.Cryptos { // Pour toutes les cryptos observées

			allElems := make(map[string]int)
			allNouveaux := make(map[string]int)
			time := time.Now()

			for i := 0; i < len(crypto.Seeds); i++ {
				// peers := parseSeeds(crypto.Code, crypto.Seeds[i])
				// logp.Info("chan <- " + strconv.Itoa(j*10+i))
				peers := <-peersChan
				// logp.Info("chan2 <- " + strconv.Itoa(j*10+i))
				newPeers := make([]string, 0)

		    elems := 0
				ok := 0
				ko := 0
				nouveaux := 0

				cryptoName, seed, peerList :=  peers[0], peers[1], peers[2:]

				if len(peerList) > 0 {
					for _, aPeer := range peerList {
						if aPeer != "" {
							elems++

							_, found := ongoingPeers[cryptoName][seed][aPeer]
							if !found {
								nouveaux++
								// logp.Info("==>"+crypto.Code+ " : "+ seed)
								ongoingPeers[cryptoName][seed][aPeer] = true
								newPeers = append(newPeers, aPeer)
							} else {
								event := beat.Event{
									Timestamp: time,
									Fields: common.MapStr{
										"seed": seed,
										"peer": aPeer,
										"crypto": cryptoName,
										"log_type": "raw",
									},
								}
								bt.client.Publish(event)
							}

							allElems[cryptoName]++
							_, found = ongoingPeers[cryptoName]["all"][aPeer]
							if !found {
								allNouveaux[cryptoName]++
								ongoingPeers[cryptoName]["all"][aPeer] = true
							}
						}
					}
				}

				for _, aPeer := range newPeers {
					// fmt.Println("->", aPeer)
					// os.Exit(1)
					go peerTester(aPeer, crypto.Port, peerTest)
				}
				// os.Exit(1)

			  for i:=0; i < nouveaux; i++ {
					testedPeer := <-peerTest
					event := beat.Event{
						Timestamp: time,
						Fields: common.MapStr{
							"seed": seed,
							"peer": testedPeer.peer,
							"available": testedPeer.status,
							"crypto": cryptoName,
							"log_type": "raw",
						},
					}
					bt.client.Publish(event)

					if testedPeer.status {
						ok++
					} else {
						ko++
					}
				}

				total[cryptoName][seed] += nouveaux

				event := beat.Event{
					Timestamp: time,
					Fields: common.MapStr{
						"crypto": cryptoName,
		        "seed": seed,
						"total": total[cryptoName][seed],
						"tailleReponse": elems,
						"live": ok,
						"dead": ko,
						"nouveaux": nouveaux,
						"pourcentUp": ((((float32)(ok)) / (float32)(elems)) * 100),
					},
				}
				bt.client.Publish(event)
				// logp.Info("Event")

				total[cryptoName]["all"] += allNouveaux[cryptoName]
				event = beat.Event{
					Timestamp: time,
					Fields: common.MapStr{
						"crypto": cryptoName,
						"seed": "all",
						"total": total[cryptoName]["all"],
						"tailleReponse": allElems[cryptoName],
						"nouveaux": allNouveaux[cryptoName],
					},
				}
				bt.client.Publish(event)
			}
		}
		logp.Info("Fin Loop")
		select {
			case <-bt.done:
				return nil
			case <-ticker.C:
				logp.Info("Boucler")
		}
	}
}

// Stop stops seedbeat.
func (bt *Seedallbeat) Stop() {
	bt.client.Close()
	close(bt.done)
}

func parseSeeds(peerResChan chan <- []string, crypto string, seed string) {

	peerRes := make([]string, 2)
	peerRes[0] = crypto
	peerRes[1] = seed

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

			if record[len(record)-2] == "A" {
				peerRes = append(peerRes, address)
		  }
		}
	}
	peerResChan <-peerRes
}

func peerTester(peer string, port string, peerTest chan <- peerTestStruct){
  conn, err := net.DialTimeout("tcp", peer+":"+port, time.Duration(1*time.Second))
	res:=true
  if err != nil {
		res=false
	} else {
	 	conn.Close()
	}
	peerTest <-peerTestStruct{peer, res}
}
