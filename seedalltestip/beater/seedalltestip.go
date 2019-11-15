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

type testingPeers struct {
	news [] string
	olds [] string
	supposedOk int
}

type testPeer struct {
	date time.Time
	isUp bool
}

var ongoingPeers = make(map[string]map[string]map[string]testPeer) //All peers

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

	bt.client, err = b.Publisher.Connect()
	if err != nil {
		return err
	}
	ticker := time.NewTicker(bt.config.Period)

	total := make(map[string]map[string]int)

	for _, crypto := range bt.config.Cryptos {    // Initialisation des structures
		// fmt.Println("->%v", crypto)
    ongoingPeers[crypto.Code] = make(map[string]map[string]testPeer)
		total[crypto.Code] = make(map[string]int)
	  for i := 0; i < len(crypto.Seeds); i++ {
			ongoingPeers[crypto.Code][crypto.Seeds[i]] = make(map[string]testPeer)
			total[crypto.Code][crypto.Seeds[i]] = 0
		}
	}

  peerTestChan := make(chan bctools.PeerTestStruct)
	peersChan := make(chan bctools.DiggedSeedStruct)

	for {
		triggerDigs(bt.config.Cryptos, peersChan)

		for _, crypto := range bt.config.Cryptos { // Pour toutes les cryptos observées
			time := time.Now()

			for i := 0; i < len(crypto.Seeds); i++ {
				// peers := parseSeeds(crypto.Code, crypto.Seeds[i])
				// logp.Info("chan <- " + strconv.Itoa(j*10+i))
				diggedPeers := <-peersChan
				peersToTest := setPeersToBeTested(diggedPeers, time)

				// logp.Info("chan2 <- " + strconv.Itoa(j*10+i))
				ok := peersToTest.supposedOk
				ko := 0
				newsOK := 0
				newsKO := 0

				for _, aPeer := range peersToTest.news {
					go bctools.PeerTester(aPeer, crypto.Port, true, peerTestChan)
				}
				for _, aPeer := range peersToTest.olds {
					go bctools.PeerTester(aPeer, crypto.Port, false, peerTestChan)
				}

				allTested := len(peersToTest.news)+len(peersToTest.olds)
			  for i:=0; i < allTested; i++ {
					testedPeer := <-peerTestChan
					ongoingPeers[diggedPeers.Crypto][diggedPeers.Seed][testedPeer.Peer] = testPeer{time, testedPeer.Status}

					emitRawEvent(bt, time, &diggedPeers, testedPeer.Peer, testedPeer.IsNew, testedPeer.Status)
					if testedPeer.Status {
						ok++
						if testedPeer.IsNew {
							newsOK++
						}
					} else {
						ko++
						if testedPeer.IsNew {
							newsKO++
						}
					}
				}

				total[diggedPeers.Crypto][diggedPeers.Seed] += len(peersToTest.news)
				pourcentUp := float32(0)
				if allTested > 0 {
					pourcentUp = (((float32)(ok)) / (float32)(allTested + peersToTest.supposedOk))
				}
				emitStdEvent(bt, time, &diggedPeers, total[diggedPeers.Crypto][diggedPeers.Seed], len(diggedPeers.Peers), allTested, ok, ko, len(peersToTest.news), newsOK, newsKO, pourcentUp)
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

func triggerDigs(cryptos [] config.Crypto, peersChan chan bctools.DiggedSeedStruct ) {
	for _, crypto := range cryptos { // Pour toutes les cryptos observées
		for i := 0; i < len(crypto.Seeds); i++ {
			// logp.Info("chan -> " + strconv.Itoa(j*10+i))
			go bctools.ParseSeeds(crypto.Code, crypto.Seeds[i], peersChan)
		}
	}
}

func setPeersToBeTested(digged bctools.DiggedSeedStruct, t time.Time) testingPeers {
	newPeers := make([]string, 0)
	oldPeers := make([]string, 0)
	supposedOk := 0
	for _, aPeer := range digged.Peers { // Résultat d'un dig
		// logp.Info("Ajout peer")
		lastPing, found := ongoingPeers[digged.Crypto][digged.Seed][aPeer]
		if !found { // Never see this peer
			newPeers = append(newPeers, aPeer)
		} else {
			threshold := t.Add(time.Hour * 24 * time.Duration(-1))
			if lastPing.date.Before(threshold) {
				logp.Info("Ajout vieux peer à tester %v %v", aPeer, t)
				oldPeers = append(oldPeers, aPeer)
			} else {
				if !lastPing.isUp {
					logp.Info("Ajout peer a tester dead %v %v", aPeer, t)
					oldPeers = append(oldPeers, aPeer)
				} else {
					supposedOk++
				}
			}
		}
	}
	return testingPeers{newPeers, oldPeers, supposedOk}
}

func emitRawEvent(bt *Seedallbeat, t time.Time, dig * bctools.DiggedSeedStruct, peer string, isnew bool, available bool ) {
	event := beat.Event{
		Timestamp: t,
		Fields: common.MapStr{
			"log_type": "raw",
			"crypto": dig.Crypto,
			"seed": dig.Seed,
			"peer": peer,
			"isNew": isnew,
			"available": available,
		},
	}
	bt.client.Publish(event)
}

func emitStdEvent(bt *Seedallbeat, t time.Time, dig * bctools.DiggedSeedStruct, sum int, tailleReponse int, tailleTest int,ok int, ko int, news int, newsok int, newsko int, pourcentUp float32) {
		event := beat.Event{
			Timestamp: t,
			Fields: common.MapStr{
				"crypto": dig.Crypto,
				"seed": dig.Seed,
				"total": sum,
				"tailleReponse": tailleReponse,
				"tailleTest": tailleTest,
				"live": ok,
				"dead": ko,
				"news": news,
				"newslive": newsok,
				"newsdead": newsko,
				"pourcentUp": pourcentUp,
			},
		}
		bt.client.Publish(event)
}
