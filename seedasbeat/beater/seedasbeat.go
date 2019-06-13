package beater

import (
	"fmt"
	"time"
	"strings"

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
  ongoingPeers := make(map[string]bool)
	ongoingAs := make(map[string]As)

	for {
		select {
			case <-bt.done:
				return nil
			case <-ticker.C:
		}
		time := time.Now()
		newPeersAsn := make([]string, 1)
		newPeersAsn[0] = "js/iptoasn.js"

    for i := 0; i < len(bt.config.Seed); i++ {
			peersChan := make(chan []string)

			go parseSeeds(peersChan, bt.config.Seed[i])
			peers := <-peersChan

			peers = peers[1:]

			for _, newPeer := range peers {
				if newPeer != "" {
					_, found := ongoingPeers[newPeer]
					if !found {
						ongoingPeers[newPeer] = true
						newPeersAsn = append(newPeersAsn, newPeer)
					}
				}
			}
		}

		// logp.Info("********************** " + strings.Join(newPeersAsn, " "))
		out, _ := exec.Command("node", newPeersAsn...).Output()
		// logp.Info("********************** " + string(out))
		uniqPeers := make(map[string]bool)
		for _, line := range strings.Split(string(out), "\n") {
			if (line != "") {
				record := strings.Split(line, ";")
				elem, found := ongoingAs[record[0]] // AS Number
				if found {
					elem.counter++
				} else {
					elem.num = record[0]
					elem.name = record[1]
					elem.country = record[2]
					elem.counter = 1
				}
				ongoingAs[record[0]] = elem
				uniqPeers[record[0]] = true
			}
		}

		var value As
		for k := range uniqPeers {
			value = ongoingAs[k]
			event := beat.Event{
				Timestamp: time,
				Fields: common.MapStr{
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

// Stop stops seedbeat.
func (bt *Seedbeat) Stop() {
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
