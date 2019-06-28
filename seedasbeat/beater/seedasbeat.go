package beater

import (
	"fmt"
	"time"
	"strings"

	"github.com/elastic/beats/libbeat/beat"
	"github.com/elastic/beats/libbeat/common"
	"github.com/elastic/beats/libbeat/logp"
  "os/exec"

	"github.com/sfrenot/seedbeat/seedasbeat/config"
)

// Seedbeat configuration.
type Seedbeat struct {
	done   chan struct{}
	config config.Config
	client beat.Client
}

type As struct {
	num string
	name string
	country string
	counter int
}

// New creates an instance of seedbeat.
func New(b *beat.Beat, cfg *common.Config) (beat.Beater, error) {
	c := config.DefaultConfig
	if err := cfg.Unpack(&c); err != nil {
		return nil, fmt.Errorf("Error reading config file: %v", err)
	}
	bt := &Seedbeat{
		done:   make(chan struct{}),
		config: c,
	}
	return bt, nil
}

// Run starts seedbeat.
func (bt *Seedbeat) Run(b *beat.Beat) error {
	logp.Info("seedbeat is running! Hit CTRL-C to stop it.")
	var err error

	bt.client, err = b.Publisher.Connect()
	if err != nil {
		return err
	}
	ticker := time.NewTicker(bt.config.Period)

  ongoingPeers := make(map[string]map[string]bool)
	ongoingAs := make(map[string]map[string]As)

	for _, crypto := range bt.config.Cryptos {
		ongoingPeers[crypto.Code] = make(map[string]bool)
		ongoingAs[crypto.Code] = make(map[string]As)
	}

	for {
		select {
			case <-bt.done:
				return nil
			case <-ticker.C:
		}
		time := time.Now()

		peersChan := make(chan []string)
    for _, crypto := range bt.config.Cryptos { // Pour toutes les cryptos observées
	    for i := 0; i < len(crypto.Seeds); i++ {
				go parseSeeds(peersChan, crypto.Code, crypto.Seeds[i])
			}
		}

		for _, crypto := range bt.config.Cryptos { // Pour toutes les cryptos observées
	    for i := 0; i < len(crypto.Seeds); i++ {

				newPeersAsn := make([]string, 1)
				newPeersAsn[0] = "js/iptoasn.js"

				peers := <-peersChan
				cryptoName, _, peerList :=  peers[0], peers[1], peers[2:]

				for _, newPeer := range peerList {
					if newPeer != "" {
						_, found := ongoingPeers[cryptoName][newPeer]
						if !found {
							ongoingPeers[cryptoName][newPeer] = true
							newPeersAsn = append(newPeersAsn, newPeer)
						}
					}
				}

				// logp.Info("********************** " + strings.Join(newPeersAsn, " "))
				out, _ := exec.Command("node", newPeersAsn...).CombinedOutput()
				// logp.Info("********************** " + string(out))
				uniqPeers := make(map[string]bool)
				for _, line := range strings.Split(string(out), "\n") {
					if (line != "") {
						record := strings.Split(line, ";")
						elem, found := ongoingAs[cryptoName][record[0]] // AS Number
						if found {
							elem.counter++
						} else {
							elem.num = record[0]
							elem.name = record[1]
							elem.country = record[2]
							elem.counter = 1
						}
						ongoingAs[cryptoName][record[0]] = elem
						uniqPeers[record[0]] = true
					}
				}

				var value As
				for k := range uniqPeers {
					value = ongoingAs[cryptoName][k]
					event := beat.Event{
						Timestamp: time,
						Fields: common.MapStr{
							"crypto": cryptoName,
		          "number": value.num,
							"name": value.name,
							"country": value.country,
							"counter": value.counter,
							"total": len(ongoingPeers),
						},
					}
					bt.client.Publish(event)
				}
			}
		}
	}
}

// Stop stops seedbeat.
func (bt *Seedbeat) Stop() {
	bt.client.Close()
	close(bt.done)
}

func parseSeeds(peerResChan chan<- []string, crypto string, seed string) {
	peerRes := make([]string, 2)
	peerRes[0] = crypto
	peerRes[1] = seed

	out, err := exec.Command("dig", seed).CombinedOutput()

	if err == nil {
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

			//fmt.Println("Stitching " + address+ "-")
			peerRes = append(peerRes, address)
		}
	}
	peerResChan <- peerRes
}
