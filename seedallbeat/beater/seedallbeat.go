package beater

import (
	"fmt"
	"time"
	"strings"
  // "os"

	"github.com/elastic/beats/libbeat/beat"
	"github.com/elastic/beats/libbeat/common"
	"github.com/elastic/beats/libbeat/logp"
  "os/exec"

	"github.com/sfrenot/seedbeat/seedallbeat/config"
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
    ongoingPeers[crypto.Code] = make(map[string]map[string]bool)
		total[crypto.Code] = make(map[string]int)

	  for i := 0; i < len(crypto.Seeds); i++ {
			ongoingPeers[crypto.Code][crypto.Seeds[i]] = make(map[string]bool)
			total[crypto.Code][crypto.Seeds[i]] = 0
		}

		ongoingPeers[crypto.Code]["all"] = make(map[string]bool)
		total[crypto.Code]["all"] = 0
	}

	for {
		select {
			case <-bt.done:
				return nil
			case <-ticker.C:
		}

    for _, crypto := range bt.config.Cryptos { // Pour toutes les cryptos observées

			allElems := 0
			allNouveaux := 0
			time := time.Now()

	    for i := 0; i < len(crypto.Seeds); i++ {
				peersChan := make(chan []string)
				// peers, err := parseSeeds(bt.config.Seed[i])
				go parseSeeds(peersChan, crypto.Seeds[i])

				peers := <-peersChan

		    elems := 0
				nouveaux := 0

				seed, peers := peers[0], peers[1:]

				for _, newPeer := range peers {
					if newPeer != "" {
						// logp.Info(seed + "Peer " + newPeer + " : ")

						elems++
						_, found := ongoingPeers[crypto.Code][seed][newPeer]
						if !found {
							nouveaux++
							// logp.Info("==>"+crypto.Code+ " : "+ seed)
							ongoingPeers[crypto.Code][seed][newPeer] = true
						}

						allElems++
						_, found = ongoingPeers[crypto.Code]["all"][newPeer]
						if !found {
							allNouveaux++
							ongoingPeers[crypto.Code]["all"][newPeer] = true
						}
					}
				}
				total[crypto.Code][seed] += nouveaux
				event := beat.Event{
					Timestamp: time,
					Fields: common.MapStr{
						"crypto": crypto.Code,
		        "seed": seed,
						"total": total[crypto.Code][seed],
						"tailleReponse": elems,
						"nouveaux": nouveaux,
					},
				}
				bt.client.Publish(event)
			}

			total[crypto.Code]["all"] += allNouveaux
			event := beat.Event{
				Timestamp: time,
				Fields: common.MapStr{
					"crypto": crypto.Code,
					"seed": "all",
					"total": total[crypto.Code]["all"],
					"tailleReponse": allElems,
					"nouveaux": allNouveaux,
				},
			}
			bt.client.Publish(event)
		}
	}
}

// Stop stops seedbeat.
func (bt *Seedallbeat) Stop() {
	bt.client.Close()
	close(bt.done)
}

func parseSeeds(peerResChan chan<- []string, seed string) {
	out, err := exec.Command("dig", seed).Output()
	if err != nil {
		return
	}
	digString := string(out)
	digLines := strings.Split(digString, "\n")
	//fmt.Println(len(digLines))

	peerRes := make([]string, 1)
	peerRes[0] = seed
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

		//fmt.Println("Stitching " + address+ "-")
		peerRes = append(peerRes, address)
	}

	peerResChan <- peerRes
}
