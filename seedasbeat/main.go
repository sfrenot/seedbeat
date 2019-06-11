package main

import (
	"os"

	"github.com/sfrenot/seedbeat/seedasbeat/cmd"

	_ "github.com/sfrenot/seedbeat/seedasbeat/include"
)

func main() {
	if err := cmd.RootCmd.Execute(); err != nil {
		os.Exit(1)
	}
}
