package aggregator

import (
	"strconv"
	"strings"
	"sync"

	"github.com/ElrondNetwork/elrond-adapter/config"
	logger "github.com/ElrondNetwork/elrond-go-logger"
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
	Name() string
}

var log = logger.GetOrCreate("aggregator")

type PairData struct {
	Base            string
	Quote           string
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
			log.Error("failed to aggregate price for pair",
				"base", pair.Base,
				"quote", pair.Quote,
				"err", err.Error(),
			)
			break
		}

		lastPrice := eh.prices[pair.Base]
		pairData := PairData{
			Base:            pair.Base,
			Quote:           pair.Quote,
			PriceMultiplied: eh.MultiplyFloat64CastStr(currPrice),
		}

		log.Info("aggregated price for pair",
			"base", pair.Base,
			"quote", pair.Quote,
			"price raw", currPrice,
			"price multiplied", pairData.PriceMultiplied,
		)

		if lastPrice == 0 {
			results = append(results, pairData)
			eh.prices[pair.Base] = currPrice
			continue
		}

		if !eh.config.CheckPercentageChange {
			results = append(results, pairData)
		} else {
			percentageChange := PercentageChange(currPrice, lastPrice)
			if percentageChange >= eh.config.PercentageThreshold {
				results = append(results, pairData)
				eh.prices[pair.Base] = currPrice
			}
		}
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
				log.Debug("failed to fetch price",
					"exchange", exchange.Name(),
					"base", baseUpper,
					"quote", quoteUpper,
					"err", err.Error(),
				)
				return
			}
			prices = append(prices, price)
		}(exchange)
	}
	wg.Wait()

	if !(len(prices) >= minValidResults) {
		log.Error("failed to reach min valid results threshold",
			"err", NotEnoughDataToComputeErr.Error(),
		)
		return -1, NotEnoughDataToComputeErr
	}

	medianPrice := ComputeMedian(prices)
	return TruncateFloat64(medianPrice), nil
}

func (eh *ExchangeAggregator) MultiplyFloat64CastStr(val float64) string {
	multiplied := uint64(val * float64(eh.config.MultiplicationPrecision))
	return strconv.FormatUint(multiplied, 10)
}
