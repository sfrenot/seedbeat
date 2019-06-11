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
	total := make(map[string]int)

  for i := 0; i < len(bt.config.Seed); i++ {
		ongoingPeers[bt.config.Seed[i]] = make(map[string]bool)
		total[bt.config.Seed[i]] = 0
	}
	ongoingPeers["all"] = make(map[string]bool)
	total["all"] = 0

	for {
		select {
			case <-bt.done:
				return nil
			case <-ticker.C:
		}
		allElems := 0
		allNouveaux := 0
		time := time.Now()

    for i := 0; i < len(bt.config.Seed); i++ {
			peersChan := make(chan []string)
			// peers, err := parseSeeds(bt.config.Seed[i])
			go parseSeeds(peersChan, bt.config.Seed[i])
			peers := <-peersChan

      params := []string{"js/iptoasn.js", "toto"}

      out, _ := exec.Command("node", params...).Output()
	    logp.Info("seedbeat " + string(out))

      //digString := string(out)
      //digLines := strings.Split(digString, "\n")
      ////fmt.Println(len(digLines))

      //peerRes := make([]string, 1)
      //peerRes[0] = seed
      //cpt := 0
      //for _, line := range digLines {

	    elems := 0
			nouveaux := 0

			seed, peers := peers[0], peers[1:]

			for _, newPeer := range peers {
				if newPeer != "" {
					logp.Info(seed + "Peer " + newPeer + " : ")

					elems++
					_, found := ongoingPeers[seed][newPeer]
					if !found {
						nouveaux++
						ongoingPeers[seed][newPeer] = true
					}

					allElems++
					_, found = ongoingPeers["all"][newPeer]
					if !found {
						allNouveaux++
						ongoingPeers["all"][newPeer] = true
					}
				}
			}
			total[seed] += nouveaux
			event := beat.Event{
				Timestamp: time,
				Fields: common.MapStr{
	        "seed": seed,
					"total": total[seed],
					"tailleReponse": elems,
					"nouveaux": nouveaux,
				},
			}
			bt.client.Publish(event)
		}

		total["all"] += allNouveaux
		event := beat.Event{
			Timestamp: time,
			Fields: common.MapStr{
				"seed": "all",
				"total": total["all"],
				"tailleReponse": allElems,
				"nouveaux": allNouveaux,
			},
		}
		bt.client.Publish(event)
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
