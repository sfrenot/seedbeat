package main

import (
	"os"
	"github.com/sfrenot/seedbeat/seedalltestip/cmd"
	_ "github.com/sfrenot/seedbeat/seedalltestip/include"
)

func main() {
	if err := cmd.RootCmd.Execute(); err != nil {
		os.Exit(1)
	}
}
