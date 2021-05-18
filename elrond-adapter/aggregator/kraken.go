package aggregator

import (
	"fmt"
	"strings"
)

const krakenPriceUrl = "https://api.kraken.com/0/public/Ticker?pair=%s%s"

type Kraken struct{}

type KrakenPriceRequest struct {
	Result map[string]KrakenPair `json:"result"`
}

type KrakenPair struct {
	Price []string `json:"c"`
}

func (b *Kraken) FetchPrice(base, quote string) (float64, error) {
	if strings.Contains(quote, "USD") {
		quote = QuoteFiat
	}

	var kr KrakenPriceRequest
	err := HttpGet(fmt.Sprintf(krakenPriceUrl, base, QuoteFiat), &kr)
	if err != nil {
		return -1, err
	}
	if len(kr.Result) == 0 {
		return -1, InvalidResponseDataErr
	}

	for k, v := range kr.Result {
		if strings.Contains(k, QuoteFiat) {
			return StrToFloat64(v.Price[0])
		}
	}

	return -1, NoPriceDataForTicker
}
