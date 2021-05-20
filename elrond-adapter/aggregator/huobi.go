package aggregator

import (
	"fmt"
	"strings"
)

const huobiPriceUrl = "https://api.huobi.pro/market/detail/merged?symbol=%s%s"

type Huobi struct{}

type HuobiPriceRequest struct {
	Ticker HuobiPriceTicker `json:"tick"`
}

type HuobiPriceTicker struct {
	Price float64 `json:"close"`
}

func (b *Huobi) FetchPrice(base, quote string) (float64, error) {
	if strings.Contains(quote, "USD") {
		quote = QuoteStable
	}

	var hpr HuobiPriceRequest
	err := HttpGet(fmt.Sprintf(
		huobiPriceUrl,
		strings.ToLower(base),
		strings.ToLower(quote),
	), &hpr)
	if err != nil {
		return -1, err
	}
	if hpr.Ticker.Price == 0 {
		return -1, InvalidResponseDataErr
	}
	return hpr.Ticker.Price, nil
}
