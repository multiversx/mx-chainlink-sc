package aggregator

import "errors"

var NotEnoughDataToComputeErr = errors.New("not enough data to compute result")

var NoPriceDataForTicker = errors.New("no price data for ticker")

var InvalidResponseDataErr = errors.New("invalid response data")
