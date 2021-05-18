package aggregator

import (
	"log"
	"strconv"
	"strings"
	"sync"

	"github.com/ElrondNetwork/elrond-adapter/config"
)

const (
	QuoteFiat   = "USD"
	QuoteStable = "USDT"
)

const (
	minValidResults = 3
)

type Exchange interface {
	FetchPrice(base, quote string) (float64, error)
}

type PairData struct {
	Base            string
	Quote           string
	ScAddress       string
	Endpoint        string
	PriceMultiplied string
}

type ExchangeAggregator struct {
	exchanges []Exchange
	prices    map[string]float64
	config    config.ExchangeConfig
}

var supportedExchanges = []Exchange{
	&Binance{},
	&Bitfinex{},
	&Cryptocom{},
	&Kraken{},
	&Gemini{},
	&Huobi{},
	&Hitbtc{},
	&Okex{},
}

func NewExchangeAggregator(exchangeConfig config.ExchangeConfig) *ExchangeAggregator {
	prices := make(map[string]float64)
	for _, pair := range exchangeConfig.Pairs {
		prices[pair.Base] = 0
	}
	return &ExchangeAggregator{
		exchanges: supportedExchanges,
		config:    exchangeConfig,
		prices:    prices,
	}
}

func (eh *ExchangeAggregator) GetPricesForPairs() []PairData {
	var results []PairData
	for _, pair := range eh.config.Pairs {
		currPrice, err := eh.GetPrice(pair.Base, pair.Quote)
		if err != nil {
			log.Println(err)
			break
		}

		lastPrice := eh.prices[pair.Base]
		pairData := PairData{
			Base:            pair.Base,
			Quote:           pair.Quote,
			ScAddress:       pair.ScAddress,
			Endpoint:        pair.Endpoint,
			PriceMultiplied: eh.MultiplyFloat64CastStr(currPrice),
		}

		if lastPrice == 0 || !eh.config.CheckPercentageChange {
			results = append(results, pairData)
		} else {
			percentageChange := PercentageChange(currPrice, lastPrice)
			if percentageChange >= eh.config.PercentageThreshold {
				results = append(results, pairData)
			}
		}
		eh.prices[pair.Base] = currPrice
	}
	return results
}

func (eh *ExchangeAggregator) GetPrice(base, quote string) (float64, error) {
	var wg sync.WaitGroup
	var mut sync.Mutex
	var prices []float64

	baseUpper := strings.ToUpper(base)
	quoteUpper := strings.ToUpper(quote)
	for _, exchange := range eh.exchanges {
		wg.Add(1)
		go func(exchange Exchange) {
			defer wg.Done()
			price, err := exchange.FetchPrice(baseUpper, quoteUpper)
			mut.Lock()
			defer mut.Unlock()
			if err != nil {
				return
			}
			prices = append(prices, price)
		}(exchange)
	}
	wg.Wait()

	if !(len(prices) >= minValidResults) {
		return -1, NotEnoughDataToComputeErr
	}

	medianPrice := ComputeMedian(prices)
	return TruncateFloat64(medianPrice), nil
}

func (eh *ExchangeAggregator) MultiplyFloat64CastStr(val float64) string {
	multiplied := uint64(val * float64(eh.config.MultiplicationPrecision))
	return strconv.FormatUint(multiplied, 10)
}
