// Config is put into a different package to prevent cyclic imports in case
// it is needed in several locations

package config

import "time"

type Crypto struct {
	Seeds []string `config:"seeds"`
	Code string `config:"name"`
}

type Config struct {
	Period time.Duration `config:"period"`
	Seed []string `config:"seed"`
	Cryptos []Crypto `config:"cryptos"`
}

var DefaultConfig = Config{
	Period: 1 * time.Second,
  Seed: []string{"seed.bitcoin.sipa.be"},
	Cryptos: []Crypto{
		Crypto{[]string{"seed.bitcoin.jonasschnelli.ch", "seed.bitcoinstats.com", "seed.bitnodes.io", "dnsseed.bluematt.me", "dnsseed.bitcoin.dashjr.org", "seed.btc.petertodd.org"}, "BTC"},
		Crypto{[]string{"dnsseed.dash.org", "dnsseed.dashdot.io", "dnsseed.masternode.io"}, "DASH"},
	},
}
