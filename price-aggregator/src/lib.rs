#![no_std]
#![feature(destructuring_assignment)]

elrond_wasm::imports!();
pub mod median;
mod price_aggregator_data;

use price_aggregator_data::{OracleStatus, PriceFeed, TokenPair};

#[elrond_wasm_derive::contract]
pub trait PriceAggregator {
    #[init]
    fn init(
        &self,
        payment_token: TokenIdentifier,
        query_payment_amount: Self::BigUint,
        submission_count: u32,
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
        #[payment] payment: Self::BigUint,
        #[payment_token] token: TokenIdentifier,
    ) -> SCResult<()> {
        require!(token == self.payment_token().get(), "wrong token type");
        self.add_balance(self.blockchain().get_caller(), &payment);
        Ok(())
    }

    fn add_balance(&self, to: Address, amount: &Self::BigUint) {
        self.balance()
            .entry(to)
            .or_default()
            .update(|balance| *balance += amount.clone());
    }

    fn subtract_balance(&self, from: Address, amount: &Self::BigUint) -> SCResult<()> {
        self.balance().entry(from).or_default().update(|balance| {
            require!(*balance >= *amount, "insufficient balance");
            *balance -= amount.clone();
            Ok(())
        })
    }

    #[endpoint]
    #[payable("*")]
    fn withdraw(&self, amount: Self::BigUint) -> SCResult<()> {
        let caller = self.blockchain().get_caller();
        self.subtract_balance(caller.clone(), &amount)?;
        self.send()
            .direct(&caller, &self.payment_token().get(), &amount, &[]);
        Ok(())
    }

    fn transfer(&self, from: Address, to: Address, amount: &Self::BigUint) -> SCResult<()> {
        self.subtract_balance(from, amount)?;
        self.add_balance(to, amount);
        Ok(())
    }

    #[endpoint(submit)]
    fn submit(&self, from: BoxedBytes, to: BoxedBytes, price: Self::BigUint) -> SCResult<()> {
        let token_pair = TokenPair { from, to };
        let mut submissions = self
            .submissions()
            .entry(token_pair.clone())
            .or_default()
            .get();
        submissions.insert(self.blockchain().get_caller(), price);
        self.create_new_round(token_pair, submissions)
    }

    fn create_new_round(
        &self,
        token_pair: TokenPair,
        mut submissions: MapMapper<Self::Storage, Address, Self::BigUint>,
    ) -> SCResult<()> {
        if submissions.len() as u32 > self.submission_count().get() {
            let price_feed =
                median::calculate(submissions.values().collect())?.ok_or("no submissions")?;
            self.rounds()
                .entry(token_pair)
                .or_default()
                .get()
                .push(&price_feed);
            submissions.clear();
        }
        Ok(())
    }

    #[view(myBalance)]
    fn my_balance(&self) -> Self::BigUint {
        self.get_balance(self.blockchain().get_caller())
    }

    #[view(getBalance)]
    fn get_balance(&self, address: Address) -> Self::BigUint {
        self.balance().get(&address).unwrap_or_default()
    }

    #[view(latestRoundData)]
    fn latest_round_data(&self) -> SCResult<MultiResultVec<PriceFeed<Self::BigUint>>> {
        self.transfer(
            self.blockchain().get_caller(),
            self.blockchain().get_sc_address(),
            &self.query_payment_amount().get(),
        )?;
        let decimals = self.decimals().get();
        Ok(self
            .rounds()
            .iter()
            .map(|(token_pair, round_values)| {
                let round_id = round_values.len();
                PriceFeed {
                    round_id: round_id as u32,
                    from: token_pair.from,
                    to: token_pair.to,
                    price: round_values.get(round_id),
                    decimals,
                }
            })
            .collect())
    }

    #[view(getOracles)]
    fn get_oracles(&self) -> MultiResultVec<Address> {
        self.oracle_status().keys().collect()
    }

    #[storage_mapper("payment_token")]
    fn payment_token(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;

    #[storage_mapper("query_payment_amount")]
    fn query_payment_amount(&self) -> SingleValueMapper<Self::Storage, Self::BigUint>;

    #[storage_mapper("submission_count")]
    fn submission_count(&self) -> SingleValueMapper<Self::Storage, u32>;

    #[storage_mapper("oracle_status")]
    fn oracle_status(&self) -> MapMapper<Self::Storage, Address, OracleStatus>;

    #[storage_mapper("rounds")]
    fn rounds(
        &self,
    ) -> MapStorageMapper<Self::Storage, TokenPair, VecMapper<Self::Storage, Self::BigUint>>;

    #[storage_mapper("submissions")]
    fn submissions(
        &self,
    ) -> MapStorageMapper<Self::Storage, TokenPair, MapMapper<Self::Storage, Address, Self::BigUint>>;

    #[storage_mapper("balance")]
    fn balance(&self) -> MapMapper<Self::Storage, Address, Self::BigUint>;

    #[view]
    #[storage_mapper("decimals")]
    fn decimals(&self) -> SingleValueMapper<Self::Storage, u8>;
}
