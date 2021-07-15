package aggregator

import (
	"testing"

	"github.com/stretchr/testify/require"
)

func TestCryptocom_FetchPriceCorrectInputShouldWork(t *testing.T) {
	t.Parallel()
	c := Cryptocom{}
	price, err := c.FetchPrice(okBaseTicker, USDQuote)
	require.Nil(t, err)
	require.True(t, price > 0)
}

func TestCryptocom_FetchPriceIncorrectInputShouldErr(t *testing.T) {
	t.Parallel()
	c := Cryptocom{}
	price, err := c.FetchPrice(errBaseTicker, USDQuote)
	require.Error(t, err)
	require.True(t, price == -1)
}
