#![no_std]
#![feature(destructuring_assignment)]

elrond_wasm::imports!();
pub mod median;
mod price_aggregator_data;

use price_aggregator_data::{Funds, OracleStatus, PriceFeed, TokenPair};

const RESERVE_ROUNDS: u64 = 2;
const ROUND_MAX: u64 = u64::MAX;

#[elrond_wasm_derive::contract]
pub trait PriceAggregator<BigUint: BigUintApi> {
    #[init]
    fn init(
        &self,
        payment_token: TokenIdentifier,
        query_payment_amount: BigUint,
        submission_count: u64,
        decimals: u8,
    ) -> SCResult<()> {
        self.payment_token().set(&payment_token);
        self.query_payment_amount().set(&query_payment_amount);
        self.submission_count().set(&submission_count);
        self.decimals().set(&decimals);
        Ok(())
    }

    #[endpoint]
    #[payable("*")]
    fn deposit(
        &self,
        #[payment] payment: BigUint,
        #[payment_token] token: TokenIdentifier,
    ) -> SCResult<()> {
        require!(token == self.payment_token().get(), "wrong token type");
        self.add_balance(self.blockchain().get_caller(), &mut payment);
        Ok(())
    }

    fn add_balance(&self, to: Address, amount: &BigUint) {
        self.balance()
            .entry(to)
            .or_default()
            .update(|balance| *balance += *amount);
    }

    fn subtract_balance(&self, from: Address, amount: &BigUint) -> SCResult<()> {
        self.balance().entry(from).or_default().update(|balance| {
            require!(*balance >= *amount, "insufficient balance");
            *balance -= amount.clone();
            Ok(())
        })
    }

    #[endpoint]
    #[payable("*")]
    fn withdraw(&self, amount: BigUint) -> SCResult<()> {
        let caller = self.blockchain().get_caller();
        self.subtract_balance(caller.clone(), &amount);
        self.send()
            .direct(&caller, &self.payment_token().get(), &amount, &[]);
        Ok(())
    }

    fn transfer(&self, from: Address, to: Address, amount: &BigUint) -> SCResult<()> {
        self.subtract_balance(from, amount)?;
        self.add_balance(to, amount);
        Ok(())
    }

    #[endpoint(submit)]
    fn submit(&self, from: BoxedBytes, to: BoxedBytes, price: BigUint) -> SCResult<()> {
        let token_pair = TokenPair { from, to };
        let submissions = self.submissions().get_or_default(token_pair);
        submissions.insert(self.blockchain().get_caller(), price);
        self.create_new_round(token_pair, submissions);
        Ok(())
    }

    fn create_new_round(
        &self,
        token_pair: TokenPair,
        submissions: MapMapper<Self::Storage, Address, BigUint>,
    ) {
        if (submissions.len() > self.submission_count().get()) {
            let priceFeed = median::calculate(submissions.values())?.ok_or("no submissions")?;
            self.rounds()
                .entry(token_pair)
                .get_or_default()
                .push(priceFeed);
            submissions.clear();
        }
    }

    #[view(myBalance)]
    fn my_balance(&self) -> BigUint {
        self.get_balance(self.blockchain().get_caller());
    }

    #[view(getBalance)]
    fn get_balance(&self, address: Address) -> BigUint {
        self.balance().get(address).unwrap_or_default()
    }

    #[view(getRoundData)]
    fn get_round_data(&self, round_id: u64) -> MultiResultVec<PriceFeed<BigUint>> {
        self.transfer(
            &self.blockchain().get_caller(),
            &self.get_sc_address(),
            self.query_payment_amount().get(),
        );
        self.rounds().get(&round_id).into()
    }

    #[view(latestRoundData)]
    fn latest_round_data(&self) -> MultiResultVec<PriceFeed<BigUint>> {
        self.get_round_data(self.latest_round_id().get())
    }

    #[view(getOracles)]
    fn get_oracles(&self) -> MultiResultVec<Address> {
        self.oracles().keys().collect()
    }

    #[storage_mapper("payment_token")]
    fn payment_token(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;

    #[storage_mapper("query_payment_amount")]
    fn query_payment_amount(&self) -> SingleValueMapper<Self::Storage, BigUint>;

    #[storage_mapper("submission_count")]
    fn submission_count(&self) -> SingleValueMapper<Self::Storage, u64>;

    #[storage_mapper("oracle_status")]
    fn oracle_status(&self) -> MapMapper<Self::Storage, Address, OracleStatus>;

    #[storage_mapper("rounds")]
    fn rounds(
        &self,
    ) -> MapStorageMapper<Self::Storage, TokenPair, VecMapper<Self::Storage, BigUint>>;

    #[storage_mapper("submissions")]
    fn submissions(
        &self,
    ) -> MapStorageMapper<Self::Storage, TokenPair, MapMapper<Self::Storage, Address, BigUint>>;

    #[storage_mapper("balance")]
    fn balance(&self) -> MapMapper<Self::Storage, Address, BigUint>;

    #[view]
    #[storage_mapper("decimals")]
    fn decimals(&self) -> SingleValueMapper<Self::Storage, u8>;
}
