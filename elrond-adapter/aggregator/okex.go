package aggregator

import (
	"fmt"
	"strings"
)

const okexPriceUrl = "https://www.okex.com/api/v5/market/ticker?instId=%s-%s"

type Okex struct{}

type OkexPriceRequest struct {
	Data []OkexTicker
}

type OkexTicker struct {
	Price string `json:"last"`
}

func (o *Okex) FetchPrice(base, quote string) (float64, error) {
	if strings.Contains(quote, "USD") {
		quote = QuoteStable
	}

	var opr OkexPriceRequest
	err := HttpGet(fmt.Sprintf(okexPriceUrl, base, quote), &opr)
	if err != nil {
		return -1, err
	}
	if len(opr.Data) == 0 {
		return -1, InvalidResponseDataErr
	}
	return StrToFloat64(opr.Data[0].Price)
}

func (o *Okex) Name() string {
	return "OKEx"
}
