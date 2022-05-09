# Price aggregator

## Overview

The price-aggregator smart contract keeps track of the price between multiple pairs of tokens.
Compared to the other chainlink contracts, it is a simplified and easier to use version.

## Deployment

Arguments:
- `payment_token:` - the fee token identifier
- `oracles` - the list of addresses which are allowed to submit price feed updates
- `submission_count` - the minimum number of submissions from different oracles which trigger an update of the price feed
- `decimals` - the number of decimals of the price feed
- `query_payment_amount` - the fee amount subtracted on each price feed request

## Submitting price feed updates

An oracle can submit a price feed update using one of the endpoints:
- `submit` - submit a single price feed as 3 arguments (`from`, `to` and `price`).
- `submitBatch` - submit multiple price feeds simultaneously. The number of arguments must be a multiple of 3.

## Rounds

Price feeds from multiple oracles are collected. When a certain threshold number of submissions has been reached (given by `submission_count`), a new round is created.
The price feed recorded in the round is the median value out of all submissions made.

## Prepaid fees

The endpoints which provide feed data, `latestRoundData` and `latestPriceFeed`, subtract a fee from the caller's balance.
A user can add to his balance through the `deposit` endpoint. Any unused funds can be withdrawn through the `withdraw` endpoint.

The fee is deducted once per endpoint call, regardless of how many price feeds are requested or returned.
The fee amount can be checked through the `query_payment_amount` view endpoint.
The `deposit` endpoint has an optional argument `on_behalf_of`, which allows increasing another account's balance.

## Querying the price feeds

Endpoints:
- `latestRoundData` takes no arguments and returns all the latest price feeds.
- `latestPriceFeed` takes a filter (as the `from` and `to` token identifiers) and returns a single price feed. The transaction fails if there is no price feed for the given filter.
- `latestPriceFeedOptional` behaves like `latestPriceFeed` but it returns an option so that the caller can handle the lack of a price feed.

A price feed contains:
- `round_id` - the ID of the current round (not related to the blockchain round ID)
- `from` - the first token
- `to` - the second token
- `price` - the price between the two tokens
- `decimals` - the number of decimals for the price
