package aggregator

import (
	"testing"

	"github.com/stretchr/testify/require"
)

func TestKraken_FetchPriceCorrectInputShouldWork(t *testing.T) {
	t.Parallel()
	bin := Kraken{}
	price, err := bin.FetchPrice(okBaseTicker, USDQuote)
	require.Nil(t, err)
	require.True(t, price > 0)
}

func TestKraken_FetchPriceIncorrectInputShouldErr(t *testing.T) {
	t.Parallel()
	bin := Kraken{}
	price, err := bin.FetchPrice(errBaseTicker, USDQuote)
	require.Error(t, err)
	require.True(t, price == -1)
}
