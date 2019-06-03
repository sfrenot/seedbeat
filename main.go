package main

import (
	"os"

	"github.com/sfrenot/seedbeat/cmd"

	_ "github.com/sfrenot/seedbeat/include"
)

func main() {
	if err := cmd.RootCmd.Execute(); err != nil {
		os.Exit(1)
	}
}
