package config

import (
	"os"

	logger "github.com/ElrondNetwork/elrond-go-logger"
	"github.com/pelletier/go-toml"
)

const configPath = "./config/config.toml"

var log = logger.GetOrCreate("config")

type GeneralConfig struct {
	Blockchain     BlockchainConfig
	GasConfig      GasConfig
	Server         ServerConfig
	Contract       ContractInformation
	PriceFeedBatch PriceFeedBatchConfig
	GasStation     GasStationConfig
	Exchange       ExchangeConfig
}

type BlockchainConfig struct {
	GasPrice uint64
	ProxyUrl string
	ChainID  string
	PemPath  string
}

type GasConfig struct {
	BatchTxGasLimit uint64
	FeedTxGasLimit  uint64
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
	Base  string
	Quote string
}

type PriceFeedBatchConfig struct {
	Endpoint string
	Address  string
}

type GasStationConfig struct {
	Address      string
	Endpoint     string
	TxPremium    uint8
	TargetAssets []GasTargetAsset
}

type GasTargetAsset struct {
	Ticker   string
	Decimals uint8
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
