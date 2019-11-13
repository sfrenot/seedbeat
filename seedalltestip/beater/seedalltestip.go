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

type peersToTest struct {
	peer string
	needtest bool
}

var OngoingPeers = make(map[string]map[string]map[string]bool) //All peers
var AllElems = make(map[string]int)
var AllNouveaux = make(map[string]int)

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
  // fmt.Println("%v", bt.config.Period)
	// os.Exit(0)

	// for _, crypto := range bt.config.Cryptos {
	// 	logp.Info(crypto.Code +" -> "+ strings.Join(crypto.Seeds, ","))
	// }
  // os.Exit(0)

	bt.client, err = b.Publisher.Connect()
	if err != nil {
		return err
	}
	ticker := time.NewTicker(bt.config.Period)


	total := make(map[string]map[string]int)

	for _, crypto := range bt.config.Cryptos {    // Initialisation des structures
		// fmt.Println("->%v", crypto)
    OngoingPeers[crypto.Code] = make(map[string]map[string]bool)
		total[crypto.Code] = make(map[string]int)

	  for i := 0; i < len(crypto.Seeds); i++ {
			OngoingPeers[crypto.Code][crypto.Seeds[i]] = make(map[string]bool)
			total[crypto.Code][crypto.Seeds[i]] = 0
		}

		OngoingPeers[crypto.Code]["all"] = make(map[string]bool)
		total[crypto.Code]["all"] = 0
	}

  peerTestChan := make(chan bctools.PeerTestStruct)
	peersChan := make(chan bctools.DiggedSeedStruct)
  // os.Exit(1)

	for {
		triggerDigs(bt.config.Cryptos, peersChan)

		for _, crypto := range bt.config.Cryptos { // Pour toutes les cryptos observées

			AllElems = make(map[string]int)
			AllNouveaux = make(map[string]int)

			time := time.Now()

			for i := 0; i < len(crypto.Seeds); i++ {
				// peers := parseSeeds(crypto.Code, crypto.Seeds[i])
				// logp.Info("chan <- " + strconv.Itoa(j*10+i))
				diggedPeers := <-peersChan

				// logp.Info("chan2 <- " + strconv.Itoa(j*10+i))
				ok := 0
				ko := 0
				testedPeers := 0

        peersToTest := setPeersToBeTested(diggedPeers)

				for _, aPeer := range peersToTest {
					if aPeer.needtest {
						testedPeers++
						go bctools.PeerTester(aPeer.peer, crypto.Port, peerTestChan)
					} else {
						emitRawEvent(bt, time, &diggedPeers, aPeer.peer, false) // Date à la place du false
 					}
				}

			  for i:=0; i < testedPeers; i++ {
					testedPeer := <-peerTestChan
					emitRawEvent(bt, time, &diggedPeers, testedPeer.Peer, testedPeer.Status) // if status -> date, sinon 1970 ?

					if testedPeer.Status {
						ok++
					} else {
						ko++
					}
				}

				total[diggedPeers.Crypto][diggedPeers.Seed] += len(peersToTest) //Faux : uniquement les news
				pourcentUp := float32(0)
				if len(peersToTest) > 0 {
					pourcentUp = (((float32)(ok)) / (float32)(len(peersToTest)))
				}

				emitStdEvent(bt, time, &diggedPeers, total[diggedPeers.Crypto][diggedPeers.Seed], len(peersToTest),  ok, ko, pourcentUp)

				// logp.Info("Event")
				//
				// total[diggedPeers.Crypto]["all"] += AllNouveaux[diggedPeers.Crypto]
				// event = beat.Event{
				// 	Timestamp: time,
				// 	Fields: common.MapStr{
				// 		"crypto": diggedPeers.Crypto,
				// 		"seed": "all",
				// 		"total": total[diggedPeers.Crypto]["all"],
				// 		"tailleReponse": AllElems[diggedPeers.Crypto],
				// 		"nouveaux": AllNouveaux[diggedPeers.Crypto],
				// 	},
				// }
				// bt.client.Publish(event)
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
func setPeersToBeTested(digged bctools.DiggedSeedStruct ) [] peersToTest {

	newPeers := make([]peersToTest, len(digged.Peers))

	for idx, aPeer := range digged.Peers { // Résultat d'un dig

		_, found := OngoingPeers[digged.Crypto][digged.Seed][aPeer]
		if !found {
			// logp.Info("==>"+crypto.Code+ " : "+ seed)
			OngoingPeers[digged.Crypto][digged.Seed][aPeer] = true
			newPeers[idx] = peersToTest{aPeer, true}
		} else {
			newPeers[idx] = peersToTest{aPeer, false}
		}

		// AllElems[digged.Crypto]++
		// _, found = OngoingPeers[digged.Crypto]["all"][aPeer]
		// if !found {
		// 	AllNouveaux[digged.Crypto]++
		// 	OngoingPeers[digged.Crypto]["all"][aPeer] = true
		// }
	}
	return newPeers
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

func emitStdEvent(bt *Seedallbeat, t time.Time, dig * bctools.DiggedSeedStruct, sum int, tailleReponse int, ok int, ko int, pourcentUp float32) {
		event := beat.Event{
			Timestamp: t,
			Fields: common.MapStr{
				"crypto": dig.Crypto,
				"seed": dig.Seed,
				"total": sum,
				"tailleReponse": tailleReponse,
				"live": ok,
				"dead": ko,
				// "nouveaux": len(peersToTest), //Faux... Il faut que les trues
				"pourcentUp": pourcentUp,
			},
		}
		bt.client.Publish(event)
}
