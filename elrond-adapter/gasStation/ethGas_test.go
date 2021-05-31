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
		GasLimit:            21000,
		TargetAsset:         "EGLD",
		TargetAssetDecimals: 18,
	})
	pair, err := gasDenom.GasPriceDenominated()
	require.Nil(t, err)
	require.True(t, pair.Denomination != "")
}
