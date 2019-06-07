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

	ongoingPeers := make(map[string]bool)

	bt.client, err = b.Publisher.Connect()
	if err != nil {
		return err
	}

	ticker := time.NewTicker(bt.config.Period)
	total := 0

	for {
		select {
		case <-bt.done:
			return nil
		case <-ticker.C:
		}

		peers, err := parseSeeds(bt.config.Seed)
		if err != nil {
			return err
		}

		now := time.Now()

    elems := 0
		nouveaux := 0
		fraction := 100

		for _, newPeer := range peers {
			if newPeer != "" {
				elems++
				_, found := ongoingPeers[newPeer]
				logp.Info("Peer " + newPeer + " : ")
				
				if !found {
					nouveaux++
					ongoingPeers[newPeer] = true
				}
			}
		}

		total += nouveaux

		event := beat.Event{
			Timestamp: now,
			Fields: common.MapStr{
				"total": total,
				"tailleReponse": elems,
				"nouveaux": nouveaux,
				"fraction": fraction,
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

func parseSeeds(seed string)([] string, error) {
	out, err := exec.Command("dig", seed).Output()
	if err != nil {
		return nil, err
	}
	digString := string(out)
	digLines := strings.Split(digString, "\n")
	//fmt.Println(len(digLines))

	peerRes := make([]string, 0)
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

	return peerRes, nil
}
