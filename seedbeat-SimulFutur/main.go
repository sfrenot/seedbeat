package main

import (
	"os"

	"github.com/sfrenot/seedbeat/seedbeat-SimulFutur/cmd"

	_ "github.com/sfrenot/seedbeat/seedbeat-SimulFutur/include"
)

func main() {
	if err := cmd.RootCmd.Execute(); err != nil {
		os.Exit(1)
	}
}
