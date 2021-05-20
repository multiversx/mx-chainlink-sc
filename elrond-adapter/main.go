package main

import (
	"github.com/ElrondNetwork/elrond-adapter/adapter"
	"github.com/ElrondNetwork/elrond-adapter/config"
)

func main() {
	cfg, err := config.LoadConfig()
	if err != nil {
		panic(err)
	}

	adapterFacade, err := adapter.NewAdapter(cfg)
	if err != nil {
		panic(err)
	}

	webServer, err := adapter.NewWebServer(adapterFacade)
	if err != nil {
		panic(err)
	}
	webServer.Run(cfg.Server.Port)
}
