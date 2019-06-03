package beater

import (
	"fmt"
	"time"
	"strings"
	"io"
	"os"

	"github.com/elastic/beats/libbeat/beat"
	"github.com/elastic/beats/libbeat/common"
	"github.com/elastic/beats/libbeat/logp"
  "os/exec"

	"github.com/sfrenot/seedbeat/config"
)

// Seedbeat configuration.
type Seedbeat struct {
	done   chan struct{}
	config config.Config
	client beat.Client
}

type peerInfo struct {
	address    string
	lastSeen   time.Time
	numberSeen int
}

func checkErr(e error) {
	if e != nil {
		panic(e)
	}
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

	addLog, err := os.Create("./toto.log")
	checkErr(err)
	defer addLog.Close()

	ongoingPeers := make(map[string]peerInfo)
	targets := [1]string{bt.config.Seed}


	bt.client, err = b.Publisher.Connect()
	if err != nil {
		return err
	}

	ticker := time.NewTicker(bt.config.Period)
	counter := 1
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

		for _, newPeer := range peers {
			if newPeer != "" {
				now := time.Now()
				pastPeer, ok := ongoingPeers[newPeer]
				seen := -1
				if !ok {
					seen = 1
				} else {
					seen = pastPeer.numberSeen + 1
				}
				ongoingPeers[newPeer] = peerInfo{address: newPeer, lastSeen: now, numberSeen: seen}
				logp.Info("Seen : "+newPeer+ " : " + string(seen) + "\n")
				// res := strings.NewReader(newPeer+"\t"+targets[counter%len(targets)]+"\t"+fmt.Sprintf("%d", seen)+"\t"+string(now.Format(time.RFC3339))+"\n")
				// logp.Info("--->" + res)
				_, err := io.Copy(addLog, strings.NewReader(newPeer+"\t"+targets[counter%len(targets)]+"\t1\t"+string(now.Format(time.RFC3339))+"\n"))
				checkErr(err)
				event := beat.Event{
					Timestamp: time.Now(),
					Fields: common.MapStr{
            "type": b.Info.Name,
						"time": string(now.Format(time.RFC3339)),
						"seed": targets[counter%len(targets)],
						"peer": newPeer,
						"times": seen,
					},
				}
				bt.client.Publish(event)
			}

		}

		// logp.Info("--->" + strings.Join(peers, ", "))



		logp.Info("Event sent")
		counter++
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
