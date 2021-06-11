package gasStation

import (
	"testing"

	"github.com/ElrondNetwork/elrond-adapter/aggregator"
	"github.com/ElrondNetwork/elrond-adapter/config"
	"github.com/stretchr/testify/require"
)

func TestEthGasDenominator_GasPriceDenominated(t *testing.T) {
	t.Parallel()
	exchange := aggregator.NewExchangeAggregator(config.ExchangeConfig{})
	gasDenom := NewEthGasDenominator(exchange, config.GasConfig{
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
