package gasStation

import (
	"fmt"
	"math"
	"math/big"
	"strconv"

	"github.com/ElrondNetwork/elrond-adapter/aggregator"
	"github.com/ElrondNetwork/elrond-adapter/config"
	logger "github.com/ElrondNetwork/elrond-go-logger"
)

var log = logger.GetOrCreate("gasStation")

const (
	gasNowUrl   = "https://www.gasnow.org/api/v3/gas/price"
	ethTicker   = "ETH"
	baseGwei    = "GWEI"
	quote       = "USD"
	ethDecimals = 18
)

var weiNeg = math.Pow(10, -ethDecimals)

type Response struct {
	Code uint16  `json:"code"`
	Data GasData `json:"data"`
}

type GasData struct {
	Fast     uint64 `json:"fast"`
	Standard uint64 `json:"standard"`
	Slow     uint64 `json:"slow"`
}

type GasPair struct {
	Base         string
	Quote        string
	Denomination string
	Address      string
	Endpoint     string
}

type EthGasDenominator struct {
	exchangeAggregator *aggregator.ExchangeAggregator
	gasConfig          config.GasConfig
}

func NewEthGasDenominator(
	exchangeAggregator *aggregator.ExchangeAggregator,
	gasConfig config.GasConfig,
) *EthGasDenominator {
	return &EthGasDenominator{
		exchangeAggregator: exchangeAggregator,
		gasConfig:          gasConfig,
	}
}

func (egd *EthGasDenominator) GasPricesDenominated() []GasPair {
	gasData, err := egd.gasPriceGwei()
	if err != nil {
		log.Error("failed to fetch gwei", "err", err.Error())
		return []GasPair{}
	}

	var gasPairs []GasPair
	for _, asset := range egd.gasConfig.TargetAssets {
		gasPair := GasPair{
			Base:     baseGwei,
			Quote:    asset.Ticker,
			Address:  egd.gasConfig.Address,
			Endpoint: egd.gasConfig.Endpoint,
		}

		if asset.Ticker == ethTicker {
			log.Info("found ETH target ticker, pushing without denominating", "gwei", gasData.Fast)
			gasPair.Denomination = strconv.FormatUint(gasData.Fast, 10)
			gasPairs = append(gasPairs, gasPair)
			continue
		}

		denominatedAmount, innerErr := egd.denominateForAsset(asset, gasData.Fast)
		if innerErr != nil {
			log.Error(fmt.Sprintf("failed to denominate gas for %s", asset.Ticker),
				"err", innerErr.Error(),
			)
			continue
		}
		log.Info(fmt.Sprintf("gas denomination from GWEI to %s", asset.Ticker),
			"gwei fast", gasData.Fast,
			"result", denominatedAmount,
		)
		gasPair.Denomination = denominatedAmount.String()
		gasPairs = append(gasPairs, gasPair)
	}
	return gasPairs
}

func (egd *EthGasDenominator) denominateForAsset(
	asset config.GasTargetAsset,
	gweiValue uint64,
) (*big.Int, error) {
	ethPrice, err := egd.exchangeAggregator.GetPrice(ethTicker, quote)
	if err != nil {
		return nil, err
	}
	targetPrice, err := egd.exchangeAggregator.GetPrice(asset.Ticker, quote)
	if err != nil {
		return nil, err
	}

	gweiFast := gweiValue
	gweiAsEth := float64(gweiFast) * weiNeg
	nominalValue := ethPrice * gweiAsEth
	nominalAmount := nominalValue / targetPrice

	targetUnit := math.Pow(10, float64(asset.Decimals))
	denominatedAmount := int64(nominalAmount * targetUnit)
	return big.NewInt(denominatedAmount), nil
}

func (egd *EthGasDenominator) gasPriceGwei() (GasData, error) {
	var gnr Response
	err := aggregator.HttpGet(gasNowUrl, &gnr)
	if err != nil {
		return GasData{}, err
	}
	return gnr.Data, nil
}
