# Elrond Chainlink Adapter

This service can be used by a ChainLink node as an external adapter for writing to
the [Elrond protocol](https://github.com/ElrondNetwork/elrond-go)

The external adapter allows you to configure an endpoint, account and private key to sign and send transactions.

## Install

- install go dependencies: `go install`
- build executable: `go build -o elrond-adapter`
- run `./elrond-adapter`

## Setup

Supported config environment divided in sections

Blockchain:
- `GasPrice`: the gas price used for sending a transaction
- `GasLimit`: the gas limit used for sending a transaction
- `ProxyUrl`: the proxy url used to connect to the Elrond Network
- `ChainID`: the ID of the chain your connecting to
- `PemPath`: (optional) defaults to `./config/owner.pem`

Aggregator:
- `Address`: address of the smart contract you wish to write updates to, bech32 encoded
- `Endpoint`: endpoint name that the service will write to in the provided smart contract

Server:
- `Port`: (optional) defaults to `:5000`, the webserver port

Exchange:
- `MultiplicationPrecision`: multiplication precision before writing to the smart contract
- `CheckPercentageChange`: whether to check the percentage change before pushing
- `PercentageThreshold`: required threshold to be met before pushing 
- `Pairs`: a list of pairs that the service will fetch updates for in case a bridge for recurrent price feeds is active

GasConfig

- `TargetAsset`: the asset to denominate gwei in
- `TargetAssetDecimals`: denomination precision in decimals
- `TxPremium`: (optional) signals if the gas price should have a premium cost
- `Address`: address of the smart contract to write denomination to, bech32 encoded
- `Endpoint`: endpoint name that the service will write to in the provided smart contract

Set the required environment, and run from the project root:

The environment variables are read from `./config/config.toml`:

```toml
[Blockchain]
    GasPrice = 1_000_000_000
    GasLimit = 100_000_000
    ProxyUrl = "https://testnet-gateway.elrond.com"
    ChainID = "T"
    PemPath = "./config/owner.pem"

[Contract]
    Address = "erd1qqqqqqqqqqqqqpgq2j35zktgvhazpvzrr9m8649gnqz53uydu00sflu9rz"
    Endpoint = "submit"

[Server]
    Port = ":5000"

[Exchange]
    MultiplicationPrecision = 100
    CheckPercentageChange = false
    PercentageThreshold = 2.5
    Pairs = [
        {Base = "EGLD", Quote= "USD", ScAddress = "erd1qqqqqqqqqqqqqpgq2j35zktgvhazpvzrr9m8649gnqz53uydu00sflu9rz", Endpoint = "submit"},
        {Base = "ETH", Quote= "USD", ScAddress = "erd1qqqqqqqqqqqqqpgq2j35zktgvhazpvzrr9m8649gnqz53uydu00sflu9rz", Endpoint = "submit"},
        {Base = "AAVE", Quote= "USD", ScAddress = "erd1qqqqqqqqqqqqqpgq2j35zktgvhazpvzrr9m8649gnqz53uydu00sflu9rz", Endpoint = "submit"},
        {Base = "LINK", Quote= "USD", ScAddress = "erd1qqqqqqqqqqqqqpgq2j35zktgvhazpvzrr9m8649gnqz53uydu00sflu9rz", Endpoint = "submit"},
        {Base = "BTC", Quote= "USD", ScAddress = "erd1qqqqqqqqqqqqqpgq2j35zktgvhazpvzrr9m8649gnqz53uydu00sflu9rz", Endpoint = "submit"},
    ]

[GasConfig]
    TargetAsset = "EGLD"
    TargetAssetDecimals = 18
    TxPremium = 0
    Address = "erd1qqqqqqqqqqqqqpgq2j35zktgvhazpvzrr9m8649gnqz53uydu00sflu9rz"
    Endpoint = "submit"
```

## API

### HTTP `POST /write` endpoint

Sends transaction and writes the request data to the Elrond network

Input:

```json
{
  "id": "bbfd3e3a8aed4d46abb0a89764951bf9",
  "data": {
    "value": "15051",
    "data": {},
    "sc_address": "erd1...",
    "function": "submit_endpoint",
    "round_id": "145"
  }
}
```

Output:

```json
{
  "jobRunID": "bbfd3e3a8aed4d46abb0a89764951bf9",
  "data": {
    "result": "19feccf4b8590bcc9554ad632ff23f8344d0318fbac643bdba5fa7a605373bf4"
  },
  "result": "19feccf4b8590bcc9554ad632ff23f8344d0318fbac643bdba5fa7a605373bf4",
  "statusCode": 200
}
```

### HTTP `POST /price-job` endpoint

Starts a price feed job which aggregates feeds from multiple sources and pushes data in the aggregator smart contract

Data body can be left empty, it reads input values from `config.toml`

Input:

```json
{
  "id": "bbfd3e3a8aed4d46abb0a89764951bf9",
  "data": {}
}
```

Output:

```json
{
  "jobRunID": "bbfd3e3a8aed4d46abb0a89764951bf9",
  "data": {
    "result": {
      "txHashes": [
        "25d1731151692cd75aa605dcad376c6acf0cd22d6fe0a1ea50a8e2cd25c16f27",
        "f95060ff47bc676f63a72cc5a51ead7ebbb1a21131d60e2273d5148a2fea3d95",
        "3a3092ba6bf49ad54afbdb2b08efa91b6b024e25753797dee675091c9b8f1891",
        "102ff3ef391cb4c53de2b9c672a98a4dca0c93da53be7255c827c60c8da029d3",
        "9c0c4c1ab8372efc21c4bbcadfc79162564e9895c91f73d942cb96be53ddd27e"
      ]
    }
  },
  "result": {
    "txHashes": [
      "25d1731151692cd75aa605dcad376c6acf0cd22d6fe0a1ea50a8e2cd25c16f27",
      "f95060ff47bc676f63a72cc5a51ead7ebbb1a21131d60e2273d5148a2fea3d95",
      "3a3092ba6bf49ad54afbdb2b08efa91b6b024e25753797dee675091c9b8f1891",
      "102ff3ef391cb4c53de2b9c672a98a4dca0c93da53be7255c827c60c8da029d3",
      "9c0c4c1ab8372efc21c4bbcadfc79162564e9895c91f73d942cb96be53ddd27e"
    ]
  },
  "statusCode": 200
}
```

### HTTP `POST /ethgas/denominate` endpoint

Fetched latest eth gas prices, in gwei and denominates the value in a specified asset. e.g GWEI/EGLD

Data body can be left empty, it reads input values from `config.toml`

Input:

```json
{
  "id": "bbfd3e3a8aed4d46abb0a89764951bf9",
  "data": {}
}
```

Output:

```json
{
  "jobRunID": "bbfd3e3a8aed4d46abb0a89764951bf9",
  "data": {
    "result": "19feccf4b8590bcc9554ad632ff23f8344d0318fbac643bdba5fa7a605373bf4"
  },
  "result": "19feccf4b8590bcc9554ad632ff23f8344d0318fbac643bdba5fa7a605373bf4",
  "statusCode": 200
}
```

