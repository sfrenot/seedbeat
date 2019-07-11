package main

import (
	"os"

	"github.com/sfrenot/seedbeat/seedbeat2/cmd"

	_ "github.com/sfrenot/seedbeat/seedbeat2/include"
)

func main() {
	if err := cmd.RootCmd.Execute(); err != nil {
		os.Exit(1)
	}
}
