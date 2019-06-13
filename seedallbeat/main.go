package main

import (
	"os"

	"github.com/sfrenot/seedbeat/seedallbeat/cmd"

	_ "github.com/sfrenot/seedbeat/seedallbeat/include"
)

func main() {
	if err := cmd.RootCmd.Execute(); err != nil {
		os.Exit(1)
	}
}
