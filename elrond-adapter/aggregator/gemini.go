package aggregator

import (
	"fmt"
	"strings"
)

const geminiPriceUrl = "https://api.gemini.com/v1/pubticker/%s%s"

type Gemini struct{}

type GeminiPriceRequest struct {
	Price string `json:"last"`
}

func (b *Gemini) FetchPrice(base, quote string) (float64, error) {
	if strings.Contains(quote, "USD") {
		quote = QuoteFiat
	}

	var gpr GeminiPriceRequest
	err := HttpGet(fmt.Sprintf(geminiPriceUrl, base, QuoteFiat), &gpr)
	if err != nil {
		return -1, err
	}
	if gpr.Price == "" {
		return -1, InvalidResponseDataErr
	}
	return StrToFloat64(gpr.Price)
}
