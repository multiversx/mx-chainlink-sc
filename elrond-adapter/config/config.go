package config

import (
	"os"

	logger "github.com/ElrondNetwork/elrond-go-logger"
	"github.com/pelletier/go-toml"
)

const configPath = "./config/config.toml"

var log = logger.GetOrCreate("config")

type GeneralConfig struct {
	Blockchain BlockchainInformation
	Contract   ContractInformation
	Server     ServerConfig
	Exchange   ExchangeConfig
}

type BlockchainInformation struct {
	GasPrice uint64
	GasLimit uint64
	ProxyUrl string
	ChainID  string
	PemPath  string
}

type ContractInformation struct {
	Address  string
	Endpoint string
}

type ServerConfig struct {
	Port string
}

type ExchangeConfig struct {
	MultiplicationPrecision uint32
	CheckPercentageChange   bool
	PercentageThreshold     float64
	Pairs                   []PairsConfig
}

type PairsConfig struct {
	Base      string
	Quote     string
	ScAddress string
	Endpoint  string
}

func LoadConfig() (GeneralConfig, error) {
	configFile, err := os.Open(configPath)
	if err != nil {
		return GeneralConfig{}, err
	}
	defer func(configFile *os.File) {
		err = configFile.Close()
		if err != nil {
			log.Error("failure closing file reader", "err", err.Error())
		}
	}(configFile)

	config := &GeneralConfig{}
	err = toml.NewDecoder(configFile).Decode(config)
	if err != nil {
		return GeneralConfig{}, err
	}

	return *config, nil
}
