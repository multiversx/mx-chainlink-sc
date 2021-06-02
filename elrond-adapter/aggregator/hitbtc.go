package aggregator

import (
	"fmt"
	"strings"
)

const hitbtcPriceUrl = "https://api.hitbtc.com/api/2/public/ticker/%s%s"

type Hitbtc struct{}

type HitbtcPriceRequest struct {
	Price string `json:"last"`
}

func (h *Hitbtc) FetchPrice(base, quote string) (float64, error) {
	if strings.Contains(quote, "USD") {
		quote = QuoteFiat
	}

	var hpr HitbtcPriceRequest
	err := HttpGet(fmt.Sprintf(hitbtcPriceUrl, base, quote), &hpr)
	if err != nil {
		return -1, err
	}
	if hpr.Price == "" {
		return -1, InvalidResponseDataErr
	}
	return StrToFloat64(hpr.Price)
}

func (h *Hitbtc) Name() string {
	return "HitBTC"
}
