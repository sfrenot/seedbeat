package beater

import (
	"fmt"
	"time"
	"strings"
	"encoding/json"
	// "os"

	"github.com/elastic/beats/libbeat/beat"
	"github.com/elastic/beats/libbeat/common"
	"github.com/elastic/beats/libbeat/logp"
  "os/exec"

	"github.com/sfrenot/seedbeat/seedbeat2/config"
)

// Seedbeat configuration.
type Seedbeat struct {
	done   chan struct{}
	config config.Config
	client beat.Client
}

type GeoIP struct {
	Range [2] int
	Country string
	Region string
	Eu string
	Timezone string
	City string
	Ll [2] float64
	Metro int
	Area int
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

	for {
		time := time.Now()

		peersChan := make(chan []string)
    for _, crypto := range bt.config.Cryptos { // Pour toutes les cryptos observées
	    for i := 0; i < len(crypto.Seeds); i++ {
				go parseSeeds(peersChan, crypto.Code, crypto.Seeds[i])
			}
		}

		for _, crypto := range bt.config.Cryptos { // Pour toutes les cryptos observées


			for i := 0; i < len(crypto.Seeds); i++ {
				peers := <-peersChan
				cryptoName, _, peerList :=  peers[0], peers[1], peers[2:]

				if len(peerList) > 0 {
					for _, newPeer := range peerList {
						if newPeer != "" {
							geoipCall := make([]string, 2)
							geoipCall[0] = "js/geoip.js"
							geoipCall[1] = newPeer

							out, _ := exec.Command("node", geoipCall...).CombinedOutput()

							var descIP GeoIP
							json.Unmarshal([]byte(out), &descIP)
							// fmt.Println(err)
							// logp.Info("->" + descIP.Country + " - " + descIP.City)
							// os.Exit(1)


							event := beat.Event{
								Timestamp: time,
								Fields: common.MapStr{
					        "crypto": cryptoName,
									"ip": newPeer,
									"geopoint": descIP.Ll,
								},
							}
							bt.client.Publish(event)

						}
					}
				}
			}
		}
		select {
			case <-bt.done:
				return nil
			case <-ticker.C:
		}
	}
}

// Stop stops seedbeat.
func (bt *Seedbeat) Stop() {
	bt.client.Close()
	close(bt.done)
}

func parseSeeds(peerResChan chan <- []string, crypto string, seed string) {

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
	peerResChan <-peerRes
}
