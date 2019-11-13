package beater

import (
	"fmt"
	"time"

  // "os"
	// "log"
	// "strconv"

	"github.com/elastic/beats/libbeat/beat"
	"github.com/elastic/beats/libbeat/common"
	"github.com/elastic/beats/libbeat/logp"

	"github.com/sfrenot/seedbeat/seedalltestip/config"
	"github.com/sfrenot/seedbeat/seedalltestip/beater/bctools"
)

// Seedallbeat configuration.
type Seedallbeat struct {
	done   chan struct{}
	config config.Config
	client beat.Client
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

  peerTest := make(chan bctools.PeerTestStruct)
	peersChan := make(chan bctools.DiggedSeedStruct)
  // os.Exit(1)

	for {
    for _, crypto := range bt.config.Cryptos { // Pour toutes les cryptos observées
	    for i := 0; i < len(crypto.Seeds); i++ {
				// logp.Info("chan -> " + strconv.Itoa(j*10+i))
				go bctools.ParseSeeds(crypto.Code, crypto.Seeds[i], peersChan)
			}
		}

		for _, crypto := range bt.config.Cryptos { // Pour toutes les cryptos observées

			allElems := make(map[string]int)
			allNouveaux := make(map[string]int)
			time := time.Now()

			for i := 0; i < len(crypto.Seeds); i++ {
				// peers := parseSeeds(crypto.Code, crypto.Seeds[i])
				// logp.Info("chan <- " + strconv.Itoa(j*10+i))
				diggedPeers := <-peersChan
				// logp.Info("chan2 <- " + strconv.Itoa(j*10+i))
				newPeers := make([]string, 0)

		    elems := 0
				ok := 0
				ko := 0

				// diggedPeers.Crypto, seed, peerList :=  peers[0], peers[1], peers[2:]

				for _, aPeer := range diggedPeers.Peers { // Résultat d'un dig
					if aPeer != "" {
						elems++

						_, found := ongoingPeers[diggedPeers.Crypto][diggedPeers.Seed][aPeer]
						if !found {
							// logp.Info("==>"+crypto.Code+ " : "+ seed)
							ongoingPeers[diggedPeers.Crypto][diggedPeers.Seed][aPeer] = true
							newPeers = append(newPeers, aPeer)
						} else {
							event := beat.Event{
								Timestamp: time,
								Fields: common.MapStr{
									"seed": diggedPeers.Seed,
									"peer": aPeer,
									"crypto": diggedPeers.Crypto,
									"log_type": "raw",
								},
							}
							bt.client.Publish(event)
						}

						allElems[diggedPeers.Crypto]++
						_, found = ongoingPeers[diggedPeers.Crypto]["all"][aPeer]
						if !found {
							allNouveaux[diggedPeers.Crypto]++
							ongoingPeers[diggedPeers.Crypto]["all"][aPeer] = true
						}
					}
				}


				for _, aPeer := range newPeers {
					go bctools.PeerTester(aPeer, crypto.Port, peerTest)
				}

			  for i:=0; i < len(newPeers); i++ {
					testedPeer := <-peerTest
					event := beat.Event{
						Timestamp: time,
						Fields: common.MapStr{
							"seed": diggedPeers.Seed,
							"peer": testedPeer.Peer,
							"available": testedPeer.Status,
							"crypto": diggedPeers.Crypto,
							"log_type": "raw",
						},
					}
					bt.client.Publish(event)

					if testedPeer.Status {
						ok++
					} else {
						ko++
					}
				}

				total[diggedPeers.Crypto][diggedPeers.Seed] += len(newPeers)
				pourcentUp := float32(0)
				if len(newPeers) > 0 {
					pourcentUp = (((float32)(ok)) / (float32)(len(newPeers)))
				}

				event := beat.Event{
					Timestamp: time,
					Fields: common.MapStr{
						"crypto": diggedPeers.Crypto,
		        "seed": diggedPeers.Seed,
						"total": total[diggedPeers.Crypto][diggedPeers.Seed],
						"tailleReponse": elems,
						"live": ok,
						"dead": ko,
						"nouveaux": len(newPeers),
						"pourcentUp": pourcentUp,
					},
				}
				bt.client.Publish(event)

				// logp.Info("Event")
				//
				// total[diggedPeers.Crypto]["all"] += allNouveaux[diggedPeers.Crypto]
				// event = beat.Event{
				// 	Timestamp: time,
				// 	Fields: common.MapStr{
				// 		"crypto": diggedPeers.Crypto,
				// 		"seed": "all",
				// 		"total": total[diggedPeers.Crypto]["all"],
				// 		"tailleReponse": allElems[diggedPeers.Crypto],
				// 		"nouveaux": allNouveaux[diggedPeers.Crypto],
				// 	},
				// }
				// bt.client.Publish(event)
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
