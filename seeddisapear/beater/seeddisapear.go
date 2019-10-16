package beater

import (
	"fmt"
	"time"
	"strings"
  // "os"G
	// "log"
	"strconv"

	"github.com/elastic/beats/libbeat/beat"
	"github.com/elastic/beats/libbeat/common"
	"github.com/elastic/beats/libbeat/logp"
  "os/exec"

	"github.com/sfrenot/seedbeat/seeddisapear/config"
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

	allSeenPeers := make(map[string]map[string]bool)
	activePeers2hours := make(map[string]map[string]time.Time)
	total := make(map[string]int)

	for _, crypto := range bt.config.Cryptos {    // Initialisation des structures
    allSeenPeers[crypto.Code] = make(map[string]bool)
		activePeers2hours[crypto.Code] = make(map[string]time.Time)
		total[crypto.Code] = 0
	}

	for {
		peersChan := make(chan []string)
    for _, crypto := range bt.config.Cryptos { // Pour toutes les cryptos observées
	    for i := 0; i < len(crypto.Seeds); i++ {
				// logp.Info("chan -> " + strconv.Itoa(j*10+i))
				go parseSeeds(peersChan, crypto.Code, crypto.Seeds[i])
			}
		}

		for _, crypto := range bt.config.Cryptos { // Pour toutes les cryptos observées
			allElems := make(map[string]int)
			allNouveaux := make(map[string]int)
			allRefreshed := make(map[string]int) // Refreshed into the bucket
			allRecalled := make(map[string]int)  // Recalled from past
			allRemoved := make(map[string]int)
			temps := time.Now()

			for i := 0; i < len(crypto.Seeds); i++ {

				// peers := parseSeeds(crypto.Code, crypto.Seeds[i])
				// logp.Info("chan <- " + strconv.Itoa(j*10+i))
				peers := <-peersChan
				// logp.Info("chan2 <- " + strconv.Itoa(j*10+i))

				cryptoName, _, peerList :=  peers[0], peers[1], peers[2:]

				if len(peerList) > 0 {

					for _, newPeer := range peerList {
						if newPeer != "" {
							// logp.Info(seed + "Peer " + newPeer + " : ")

							allElems[cryptoName]++

							_, found := allSeenPeers[cryptoName][newPeer]
							if !found { // Never Seen
								allNouveaux[cryptoName]++
								allSeenPeers[cryptoName][newPeer] = true
							} else {
								_, exists := activePeers2hours[cryptoName][newPeer]
								if exists { // refresh buffer
									allRefreshed[cryptoName]++
								} else { // Recalled from past
									allRecalled[cryptoName]++
								}
							}
							activePeers2hours[cryptoName][newPeer] = temps

						}
					}
				}

        outdated := make([]string, 20)
				for peer, evnTime := range activePeers2hours[cryptoName] {
					threshold := temps.Add(time.Hour * time.Duration(-2))
					if evnTime.Before(threshold) {
						allRemoved[cryptoName]++
						outdated = append(outdated, peer)
					}
				}
				for _, peer := range outdated {
					delete(activePeers2hours[cryptoName], peer)
				}

			}
			total[crypto.Code] += allNouveaux[crypto.Code]

			logp.Info("->" + crypto.Code + " : " + temps.Format("18:04:05") +
			  " Total ever seen: " + strconv.Itoa(total[crypto.Code]) +
				" Total this loop: " + strconv.Itoa(allElems[crypto.Code]) +
				" Added " + strconv.Itoa(allNouveaux[crypto.Code]) +
				" Taille buffer " + strconv.Itoa(len(activePeers2hours[crypto.Code])) +
				" Refresh " + strconv.Itoa(allRefreshed[crypto.Code]) +
 				" Recalled " + strconv.Itoa(allRecalled[crypto.Code]) +
				" Removed " + strconv.Itoa(allRemoved[crypto.Code]))

			event := beat.Event{
				Timestamp: temps,
				Fields: common.MapStr{
					"crypto": crypto.Code,
					"totalEverSeen": total[crypto.Code],
					"totalLoop": allElems[crypto.Code],
					"added": allNouveaux[crypto.Code],
					"bufferSize": len(activePeers2hours[crypto.Code]),
					"Refreshed": allRefreshed[crypto.Code],
					"Recalled": allRecalled[crypto.Code],
					"Removed": allRemoved[crypto.Code],
				},
			}
			bt.client.Publish(event)
			// logp.Info("Event")

		}

		// logp.Info("Sortie Loop")
		select {
			case <-bt.done:
				return nil
			case <-ticker.C:
				// logp.Info("Ticker")
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

			//fmt.Println("Stitching " + address+ "-")
			peerRes = append(peerRes, address)
		}
	}
	peerResChan <-peerRes
}
