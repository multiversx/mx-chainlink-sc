package aggregator

import (
	"encoding/json"
	"io/ioutil"
	"math"
	"net/http"
	"sort"
	"strconv"
)

const (
	httpGetVerb = "GET"
)

func HttpGet(url string, castTarget interface{}) error {
	client := &http.Client{}
	req, err := http.NewRequest(httpGetVerb, url, nil)
	if err != nil {
		return err
	}
	resp, err := client.Do(req)
	if err != nil {
		return err
	}
	respBytes, err := ioutil.ReadAll(resp.Body)
	if err != nil {
		return nil
	}
	return json.Unmarshal(respBytes, castTarget)
}

func ComputeMedian(nums []float64) float64 {
	sort.Float64s(nums)

	numsLen := len(nums)
	mid := numsLen / 2

	if numsLen&1 != 0 {
		return nums[mid]
	}

	return (nums[mid-1] + nums[mid]) / 2
}

func StrToFloat64(v string) (float64, error) {
	vFloat, err := strconv.ParseFloat(v, 64)
	if err != nil {
		return -1, err
	}

	return vFloat, nil
}

func TruncateFloat64(v float64) float64 {
	return math.Round(v*100) / 100
}

func PercentageChange(curr, last float64) float64 {
	return math.Abs((curr-last)/last) * 100
}
