package aggregator

import (
	"errors"
	"fmt"
	"strings"
)

const cryptocomPriceUrl = "https://api.crypto.com/v2/public/get-ticker?instrument_name=%s_%s"

type Cryptocom struct{}

type CryptocomPriceRequest struct {
	Result CryptocomData `json:"result"`
}

type CryptocomData struct {
	Data CryptocomPair `json:"data"`
}

type CryptocomPair struct {
	Price float64 `json:"a"`
}

func (b *Cryptocom) FetchPrice(base, quote string) (float64, error) {
	if strings.Contains(quote, "USD") {
		quote = QuoteStable
	}

	var cpr CryptocomPriceRequest
	err := HttpGet(fmt.Sprintf(cryptocomPriceUrl, base, QuoteStable), &cpr)
	if err != nil {
		return -1, err
	}
	if cpr.Result.Data.Price == 0 {
		return -1, errors.New("")
	}
	return cpr.Result.Data.Price, nil
}
