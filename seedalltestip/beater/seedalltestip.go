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
	newOnes int
	peers [] string
}

var ongoingPeers = make(map[string]map[string]map[string]time.Time) //All peers

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
    ongoingPeers[crypto.Code] = make(map[string]map[string]time.Time)
		total[crypto.Code] = make(map[string]int)
	  for i := 0; i < len(crypto.Seeds); i++ {
			ongoingPeers[crypto.Code][crypto.Seeds[i]] = make(map[string]time.Time)
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

				// logp.Info("chan2 <- " + strconv.Itoa(j*10+i))
				ok := 0
				ko := 0

        peersToTest := setPeersToBeTested(diggedPeers, time)

				for _, aPeer := range peersToTest.peers {
					go bctools.PeerTester(aPeer, crypto.Port, peerTestChan)
				}

			  for i:=0; i < len(peersToTest.peers); i++ {
					testedPeer := <-peerTestChan
					emitRawEvent(bt, time, &diggedPeers, testedPeer.Peer, testedPeer.Status) // if status -> date, sinon 1970 ?
					if testedPeer.Status {
						ok++
					} else {
						ko++
					}
				}

				total[diggedPeers.Crypto][diggedPeers.Seed] += peersToTest.newOnes
				pourcentUp := float32(0)
				if len(peersToTest.peers) > 0 {
					pourcentUp = (((float32)(ok)) / (float32)(len(peersToTest.peers)))
				}
				emitStdEvent(bt, time, &diggedPeers, total[diggedPeers.Crypto][diggedPeers.Seed], len(peersToTest.peers), ok, ko, peersToTest.newOnes, pourcentUp)
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
	newPeers := make([]string, len(digged.Peers))
	new := 0
	for _, aPeer := range digged.Peers { // Résultat d'un dig
		lastPing, found := ongoingPeers[digged.Crypto][digged.Seed][aPeer]
		if !found { // Never see this peer
			new++
			newPeers = append(newPeers, aPeer)
		} else {
			threshold := t.Add(time.Hour * 24 * time.Duration(-1))
			if lastPing.Before(threshold) {
				newPeers = append(newPeers, aPeer)
			}
		}
		ongoingPeers[digged.Crypto][digged.Seed][aPeer] = t
	}

	return testingPeers{new, newPeers}
}

func emitRawEvent(bt *Seedallbeat, t time.Time, dig * bctools.DiggedSeedStruct, peer string, available bool ) {
	event := beat.Event{
		Timestamp: t,
		Fields: common.MapStr{
			"log_type": "raw",
			"crypto": dig.Crypto,
			"seed": dig.Seed,
			"peer": peer,
			"available": available,
		},
	}
	bt.client.Publish(event)
}

func emitStdEvent(bt *Seedallbeat, t time.Time, dig * bctools.DiggedSeedStruct, sum int, tailleReponse int, ok int, ko int, new int, pourcentUp float32) {
		event := beat.Event{
			Timestamp: t,
			Fields: common.MapStr{
				"crypto": dig.Crypto,
				"seed": dig.Seed,
				"total": sum,
				"tailleReponse": tailleReponse,
				"live": ok,
				"dead": ko,
				"nouveaux": new,
				"pourcentUp": pourcentUp,
			},
		}
		bt.client.Publish(event)
}
