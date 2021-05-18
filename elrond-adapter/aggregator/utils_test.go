package aggregator

import (
	"testing"

	"github.com/stretchr/testify/require"
)

var evenSample = []float64{57870.5, 57826.23, 57817, 57835.81, 57829.36, 57876.22}
var oddSample = []float64{57870.5, 57826.23, 57817, 57835.81, 57829.36, 57876.22, 57842.21}

func TestComputeMedian_EvenSample(t *testing.T) {
	t.Parallel()
	expected := 57832.585
	median := ComputeMedian(evenSample)
	require.True(t, median == expected)
}

func TestComputeMedia_OddSample(t *testing.T) {
	t.Parallel()
	expected := 57835.81
	median := ComputeMedian(oddSample)
	require.True(t, median == expected)
}

func TestStrToFloat64_CorrectInputShouldWork(t *testing.T) {
	t.Parallel()
	input := "12.99"
	expected := 12.99
	res, err := StrToFloat64(input)
	require.Nil(t, err)
	require.True(t, expected == res)
}

func TestStrToFloat64_IncorrectInputShouldErr(t *testing.T) {
	t.Parallel()
	input := "12,932932"
	res, err := StrToFloat64(input)
	require.Error(t, err)
	require.True(t, res == -1)
}

func TestPercentageChange(t *testing.T) {
	t.Parallel()
	expected := 2.446043165467625
	require.True(t, expected == PercentageChange(6.78, 6.95))
}
