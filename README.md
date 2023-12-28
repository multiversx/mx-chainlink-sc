# Chainlink smart contracts

This repository contains a collection of smart contracts for the MultiversX network which interact with the Chainlink ecosystem.

## Overview - smart contracts

### Price aggregator

See the [price aggregator readme](price-aggregator/README.md).

### Oracle

The Oracle is a smart contract that interacts with a chainlink node via [an external initiator](https://docs.chain.link/docs/external-initiators-introduction) and [an external adapter](https://docs.chain.link/docs/external-adapters).
It interacts with the client (in the request model), or with the aggregator (in the decentralized model). You can read more about them in the [official chainlink documentation](https://docs.chain.link/docs/architecture-overview).

#### Endpoints

callable by anyone:
  - `request` - registers a new request, which is going to be handled off-chain by an oracle node

callable by oracle nodes:
  - `fulfillRequest` - provide the answer for a request

- views:
  - `requestsAsVec` - get a list of active requests
  - `authorizedNodes` - get a list of authorized nodes

- callable by the owner:
  - `submit` - forward data to an aggregator
  - `addAuthorization` - authorize an address to act as an oracle node in order to fulfill requests
  - `removeAuthorization` - remove an authorization

### Client

The Client smart contract requests data from a single oracle. When the oracle fulfills the request, it notfies the client via a callback.

#### Endpoints

- `getClientData` - view the current results (if any)
- `sendRequest` - forwards a request to the oracle in order to be handled off-chain
- `reply` - called by the oracle upon completion of a request

### Aggregator

The Aggregator smart contract collects the data from multiple oracles and provides a single result. This result is created from taking the median values between all the received results.
The oracles are paid for each contribution and this is done by using funds which have been previously deposited into the Aggregator smart contract by any user.

#### Endpoints

- For managing deposits:
  - `addFunds` - adds funds to a deposit, so that the oracles can be paid when they fulfill requests
  - `withdrawFunds` - withdraw previously deposited funds

- callable by the owner of this smart contract:
  - `changeOracles` - updates the list of authorized oracles, their admins and several other parameters
  - `updateFutureRounds` - configures the amount paid to oracles in future rounds and a few other parameters
  - `setRequesterPermissions` - manages requester permissions; a requester may initiate new rounds

- callable by oracles:
  - `submit` - submit a set of values for a certain round; callable by oracles

- views
  - `allocatedFunds` - funds which were paid to the oracles as rewards
  - `availableFunds` - funds which are available for the aggregator smart contract in order to pay oracles which contribute
  - `oracleCount` - the number of oracles
  - `getRoundData` - get the data from a specific round
  - `latestRoundData` - get the data of the latest round
  - `withdrawablePayment` - get the sum withdrawable by a certain oracle
  - `withdrawableAddedFunds` - get the sum withdrawable from a deposit
  - `getAdmin` - get the address which acts as the given oracle's administrator
  - `oracleRoundState` - provides some details which are relevant to an oracle looking to submit data

- callable by an oracle's admin
  - `withdrawPayment` - withdraw the rewards of a managed oracle to a given address
  - `transferAdmin` - initiates a transfers of the administration rights of an oracle to another address
  - `acceptAdmin` - finalizes the transfer of administration rights of an oracle

- callable by authorized requesters
  - `requestNewRound` - initializes a new round; usually not needed since a new round begins when enough oracle results are accumulated

### Exchange

It provides an exchange between a pair of tokens at a given exchange rate. This smart contract mainly serves as an example on how to receive data from the Aggregator and how to consume it.
The exchange rate is provided from off-chain by the chainlink oracles, which then send this data to the Aggregator.

#### Endpoints

- callable by the owner:
  - `deposit` - adds liquidity to the smart contract

- callable by anyone:
  - `exchange` - payable endpoint which converts the provided token into the other token handled by the exchange; the exchange rate is fetched when this endpoint is called

## Using the data feed

In most cases, the decentralized model is what a consuming smart contract should use. For this, an Aggregator, together with several Oracles (and the coresponding oracle nodes) have to be started.

In this scenario, in order to consume the data feed provided by chainlink, you need to call the `latestRoundData` of the Aggregator smart contract.

## Testnet interaction

Inside the `interaction` folder you can find snippets which help in setting up a test scenario on the testnet.

### Setting up the testnet

First, check the guide on [how to set up a local testnet](https://docs.multiversx.com/developers/setup-local-testnet/).
Important note: these contracts rely on callback functionality which normally gets enabled only at the start of epoch 4 (controlled by the `RepairCallbackEnableEpoch` config parameter).

### Postman setup

In order to inspect the transactions resulting from the testnet you can use Postman (it can be installed via Ubuntu Software).

To obtain the transaction info use:
`http://localhost:7950/transaction/:transaction_hash?withResults=true`
The `transaction_hash` is a path variable which has to be set to the actual hash of the transaction in Postman's Path Variables section.

### Testnet snippets

In a local terminal, import the snippets by running:

```
source interaction/local-testnet-snippets.sh
```

The snippets provided in the script register some shortcuts which can be run:

```
step_1_issue_tokens
step_2_configure_tokens TOKA-9dabbe TOKB-4a18f6
step_3_deploy_sc
step_4_prepare_aggregator
step_5_prepare_exchange
step_6_send_funds_to_other_users
step_7_send_round_1
step_8_exchange_tokens
```

#### Steps - in detail

1. In step 1, we issue 2 tokens (TOKA, TOKB) which we'll then use in the exchange later on.
Inspect the transaction results to obtain the token names 
2. Step 2 sets the local variables which will be used by the other steps. Here you'll have to manually replace the token names with the ones which resulted from step 1 in the command.
3. This step deploys the smart contracts: 3 oracles, 1 aggregator and 1 exchange.
4. Prepares the aggregator - adds funds which will be used to pay the oracles, informs the aggregator about the oracles and the requester and starts a new round.
5. Deposits some ESDT tokens in the exchange smart contract.
6. Send some ESDT tokens to the users so that they can exchange funds.
7. Provide feed data regarding the exchange rate to the oracle smart contracts. The oracles forward this information automatically to the aggregator.
8. Showcase the exchanging of tokens in 2 separate transactions: from Token A to Token B, and the reverse - from Token B to Token A. The exchange is done at the exchange rate which was sent in step 7.
