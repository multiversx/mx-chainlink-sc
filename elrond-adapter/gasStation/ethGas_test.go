package gasStation

import (
	"strconv"
	"testing"

	"github.com/ElrondNetwork/elrond-adapter/aggregator"
	"github.com/ElrondNetwork/elrond-adapter/config"
	"github.com/stretchr/testify/require"
)

var getGasDenominator = func(gasStation config.GasStationConfig) *EthGasDenominator {
	exchange := aggregator.NewExchangeAggregator(config.ExchangeConfig{})
	return NewEthGasDenominator(exchange, gasStation)
}

func TestEthGasDenominator_GasPriceDenominated(t *testing.T) {
	t.Parallel()
	gasDenom := getGasDenominator(config.GasStationConfig{
		TargetAssets: []config.GasTargetAsset{
			{
				Ticker:   "EGLD",
				Decimals: 18,
			},
		},
	})
	pairs := gasDenom.GasPricesDenominated()
	require.True(t, len(pairs) == 1)
}

func TestEthGasDenominator_GasPricesDenominatedETH(t *testing.T) {
	t.Parallel()
	gasDenom := getGasDenominator(config.GasStationConfig{
		TargetAssets: []config.GasTargetAsset{
			{
				Ticker:   "ETH",
				Decimals: 18,
			},
		},
	})
	gwei, _ := gasDenom.gasPriceGwei()
	pairs := gasDenom.GasPricesDenominated()
	require.True(t, pairs[0].Value == strconv.FormatUint(gwei.Fast, 10))
}

func TestEthGasDenominator_GasPricesDenominatedMultipleAssets(t *testing.T) {
	t.Parallel()

	gasDenom := getGasDenominator(config.GasStationConfig{
		TargetAssets: []config.GasTargetAsset{
			{
				Ticker:   "EGLD",
				Decimals: 18,
			},
			{
				Ticker:   "ETH",
				Decimals: 18,
			},
		},
	})

	gwei, _ := gasDenom.gasPriceGwei()

	pairs := gasDenom.GasPricesDenominated()
	for _, pair := range pairs {
		if pair.Base == "ETH" {
			require.True(t, pair.Value == strconv.FormatUint(gwei.Fast, 10))
			continue
		}
		require.True(t, pair.Value != "")
	}
}
