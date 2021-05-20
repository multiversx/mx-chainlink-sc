package aggregator

import (
	"fmt"
	"strings"
)

const bitfinexPriceUrl = "https://api.bitfinex.com/v1/pubticker/%s%s"

type Bitfinex struct{}

type BitfinexPriceRequest struct {
	Price string `json:"last_price"`
}

func (b *Bitfinex) FetchPrice(base, quote string) (float64, error) {
	if strings.Contains(quote, "USD") {
		quote = QuoteFiat
	}

	var bit BitfinexPriceRequest
	err := HttpGet(fmt.Sprintf(bitfinexPriceUrl, base, QuoteFiat), &bit)
	if bit.Price == "" {
		err = HttpGet(fmt.Sprintf(bitfinexPriceUrl, base+":", QuoteFiat), &bit)
	}
	if err != nil {
		return -1, nil
	}
	if bit.Price == "" {
		return -1, InvalidResponseDataErr
	}
	return StrToFloat64(bit.Price)
}
