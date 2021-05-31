package adapter

import (
	"encoding/hex"
	"errors"
	"math/big"

	"github.com/ElrondNetwork/elrond-adapter/aggregator"
	"github.com/ElrondNetwork/elrond-adapter/config"
	models "github.com/ElrondNetwork/elrond-adapter/data"
	"github.com/ElrondNetwork/elrond-adapter/interaction"
	logger "github.com/ElrondNetwork/elrond-go-logger"
)

var log = logger.GetOrCreate("adapter")

type adapter struct {
	chainInteractor    *interaction.BlockchainInteractor
	exchangeAggregator *aggregator.ExchangeAggregator
	config             config.GeneralConfig
}

func NewAdapter(config config.GeneralConfig) (*adapter, error) {
	interactor, err := interaction.NewBlockchainInteractor(config.Blockchain)
	if err != nil {
		log.Error("failed initialising blockchain interactor", "err", err.Error())
		return nil, err
	}
	exchangeAggregator := aggregator.NewExchangeAggregator(config.Exchange)
	return &adapter{
		chainInteractor:    interactor,
		exchangeAggregator: exchangeAggregator,
		config:             config,
	}, nil
}

func (a *adapter) HandlePriceFeed(data models.RequestData) (string, error) {
	price, err := a.exchangeAggregator.GetPrice(data.Value, data.Result)
	if err != nil {
		return "", err
	}
	return a.exchangeAggregator.MultiplyFloat64CastStr(price), nil
}

func (a *adapter) HandlePriceFeedJob() ([]string, error) {
	var txHashes []string
	pairs := a.exchangeAggregator.GetPricesForPairs()
	for _, pair := range pairs {
		argsHex, err := prepareJobResultArgsHex(pair.Base, pair.Quote, pair.PriceMultiplied)
		if err != nil {
			log.Error("price job: failed to prepare args hex", "err", err.Error())
			break
		}
		inputData := pair.Endpoint + "@" + argsHex
		tx, err := a.chainInteractor.CreateSignedTx("0", []byte(inputData), pair.ScAddress)
		if err != nil {
			log.Error("price job: failed to sign transaction", "err", err.Error())
			break
		}
		txHash, err := a.chainInteractor.SendTx(tx)
		if err != nil {
			log.Error("price job: failed to send transaction", "err", err.Error())
			break
		}
		txHashes = append(txHashes, txHash)
	}

	return txHashes, nil
}

func (a *adapter) HandleWriteFeed(data models.RequestData) (string, error) {
	scEndpoint := data.Function
	scAddress := data.ScAddress
	if scEndpoint == "" {
		scEndpoint = a.config.Contract.Endpoint
	}
	if scAddress == "" {
		scAddress = a.config.Contract.Address
	}

	argsHex, err := prepareWriteRequestArgsHex(data.Value, data.RoundID)
	if err != nil {
		log.Error("write job: failed to prepare args hex", "err", err.Error())
		return "", err
	}
	inputData := scEndpoint + "@" + argsHex
	tx, err := a.chainInteractor.CreateSignedTx("0", []byte(inputData), scAddress)
	if err != nil {
		log.Error("write job: failed to sign transaction", "err", err.Error())
		return "", err
	}

	txHash, err := a.chainInteractor.SendTx(tx)
	if err != nil {
		log.Error("write job: failed to send transaction", "err", err.Error())
		return "", err
	}

	return txHash, nil
}

func prepareJobResultArgsHex(base, quote, price string) (string, error) {
	parsedPrice, ok := big.NewInt(0).SetString(price, 10)
	if !ok {
		return "", errors.New("failure parsing price")
	}

	args := hex.EncodeToString([]byte(base)) +
		"@" + hex.EncodeToString([]byte(quote)) +
		"@" + hex.EncodeToString(parsedPrice.Bytes())

	return args, nil
}

func prepareWriteRequestArgsHex(value, roundID string) (string, error) {
	parsedValue, ok := big.NewInt(0).SetString(value, 10)
	if !ok {
		return "", errors.New("failure parsing request value")
	}
	parsedRoundID, ok := big.NewInt(0).SetString(roundID, 10)
	if !ok {
		return "", errors.New("failure parsing roundID")
	}

	args := hex.EncodeToString(parsedValue.Bytes()) +
		"@" + hex.EncodeToString(parsedRoundID.Bytes())

	return args, nil
}
