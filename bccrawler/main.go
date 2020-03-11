package main

import (
	"os"
	"github.com/sfrenot/seedbeat/bccrawler/cmd"
	_ "github.com/sfrenot/seedbeat/bccrawler/include"
)

func main() {
	if err := cmd.RootCmd.Execute(); err != nil {
		os.Exit(1)
	}
}
