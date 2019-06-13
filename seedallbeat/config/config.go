// Config is put into a different package to prevent cyclic imports in case
// it is needed in several locations

package config

import "time"

type Crypto struct {
	Seeds []string
	Code string
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
		Crypto{[]string{"seed.bitcoin.sipa.be", "seed.bitcoin.jonasschnelli.ch"}, "BTC"},
		Crypto{[]string{"dnsseed.dash.org", "dnsseed.dashdot.io", "dnsseed.masternode.io"}, "DASH"},
	},
}
