package aggregator

import (
	"testing"

	"github.com/stretchr/testify/require"
)

func TestOkex_FetchPriceCorrectInputShouldWork(t *testing.T) {
	t.Parallel()
	bin := Okex{}
	price, err := bin.FetchPrice(okBaseTicker, USDQuote)
	require.Nil(t, err)
	require.True(t, price > 0)
}

func TestOkex_FetchPriceIncorrectInputShouldErr(t *testing.T) {
	t.Parallel()
	bin := Okex{}
	price, err := bin.FetchPrice(errBaseTicker, USDQuote)
	require.Error(t, err)
	require.True(t, price == -1)
}
