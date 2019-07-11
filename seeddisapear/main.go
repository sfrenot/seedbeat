package main

import (
	"os"

	"github.com/sfrenot/seedbeat/seeddisapear/cmd"

	_ "github.com/sfrenot/seedbeat/seeddisapear/include"
)

func main() {
	if err := cmd.RootCmd.Execute(); err != nil {
		os.Exit(1)
	}
}
