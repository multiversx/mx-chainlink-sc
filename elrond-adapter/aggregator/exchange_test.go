package aggregator

import (
	"testing"

	"github.com/ElrondNetwork/elrond-adapter/config"
	"github.com/stretchr/testify/require"
)

const (
	okBaseTicker  = "ETH"
	USDQuote      = "USD"
	errBaseTicker = "ETHZ"
	egldTicker    = "EGLD"
)

func TestExchangeAggregator_GetPriceMultipliedShouldWork(t *testing.T) {
	t.Parallel()
	aggregator := NewExchangeAggregator(config.ExchangeConfig{})
	price, err := aggregator.GetPrice(okBaseTicker, USDQuote)
	require.Nil(t, err)
	require.True(t, price > 0)
}

func TestExchangeAggregator_GetPriceMultipliedShouldErr(t *testing.T) {
	t.Parallel()
	aggregator := NewExchangeAggregator(config.ExchangeConfig{})
	price, err := aggregator.GetPrice(errBaseTicker, USDQuote)
	require.Error(t, err)
	require.True(t, price == -1)
}

func TestExchangeAggregator_GetPriceMultipliedEGLD(t *testing.T) {
	t.Parallel()
	aggregator := NewExchangeAggregator(config.ExchangeConfig{})
	price, err := aggregator.GetPrice(egldTicker, USDQuote)
	require.Nil(t, err)
	require.True(t, price > 0)
}
