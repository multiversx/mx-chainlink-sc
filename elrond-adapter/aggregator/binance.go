package aggregator

import (
	"fmt"
	"strings"
)

const (
	binancePriceUrl = "https://api.binance.com/api/v3/ticker/price?symbol=%s%s"
)

type BinancePriceRequest struct {
	Symbol string `json:"symbol"`
	Price  string `json:"price"`
}

type Binance struct{}

func (b *Binance) FetchPrice(base, quote string) (float64, error) {
	if strings.Contains(quote, "USD") {
		quote = QuoteStable
	}

	var bpr BinancePriceRequest
	err := HttpGet(fmt.Sprintf(binancePriceUrl, base, QuoteStable), &bpr)
	if err != nil {
		return -1, err
	}
	if bpr.Price == "" {
		return -1, InvalidResponseDataErr
	}
	return StrToFloat64(bpr.Price)
}

func (b *Binance) Name() string {
	return "Binance"
}
