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
        oracles: Vec<Address>,
        submission_count: u32,
        decimals: u8,
        query_payment_amount: Self::BigUint,
    ) -> SCResult<()> {
        self.payment_token().set(&payment_token);
        self.query_payment_amount().set(&query_payment_amount);
        self.submission_count().set(&submission_count);
        self.decimals().set(&decimals);
        oracles.iter().for_each(|oracle| {
            self.oracle_status().insert(
                oracle.clone(),
                OracleStatus {
                    total_submissions: 0,
                    accepted_submissions: 0,
                },
            );
        });
        Ok(())
    }

    #[endpoint]
    #[payable("*")]
    fn deposit(
        &self,
        #[payment] payment: Self::BigUint,
        #[payment_token] token: TokenIdentifier,
        #[var_args] on_behalf_of: OptionalArg<Address>,
    ) -> SCResult<()> {
        require!(token == self.payment_token().get(), "wrong token type");
        let to = on_behalf_of
            .into_option()
            .unwrap_or_else(|| self.blockchain().get_caller());
        self.add_balance(to, &payment);
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

    #[endpoint]
    fn submit(&self, from: BoxedBytes, to: BoxedBytes, price: Self::BigUint) -> SCResult<()> {
        self.require_is_oracle()?;
        let token_pair = TokenPair { from, to };
        let mut submissions = self
            .submissions()
            .entry(token_pair.clone())
            .or_default()
            .get();
        let accepted = submissions
            .insert(self.blockchain().get_caller(), price)
            .is_none();
        self.oracle_status()
            .entry(self.blockchain().get_caller())
            .and_modify(|oracle_status| {
                oracle_status.accepted_submissions += accepted as u64;
                oracle_status.total_submissions += 1;
            });
        self.create_new_round(token_pair, submissions)?;
        Ok(())
    }

    fn require_is_oracle(&self) -> SCResult<()> {
        require!(
            self.oracle_status()
                .contains_key(&self.blockchain().get_caller()),
            "only oracles allowed"
        );
        Ok(())
    }

    fn create_new_round(
        &self,
        token_pair: TokenPair,
        mut submissions: MapMapper<Self::Storage, Address, Self::BigUint>,
    ) -> SCResult<()> {
        if submissions.len() as u32 >= self.submission_count().get() {
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
        self.subtract_query_payment()?;
        require!(!self.rounds().is_empty(), "no completed rounds");
        Ok(self
            .rounds()
            .iter()
            .map(|(token_pair, round_values)| self.make_price_feed(token_pair, round_values))
            .collect())
    }

    #[endpoint(latestPriceFeed)]
    fn latest_price_feed(
        &self,
        from: BoxedBytes,
        to: BoxedBytes,
    ) -> SCResult<MultiArg5<u32, BoxedBytes, BoxedBytes, Self::BigUint, u8>> {
        self.subtract_query_payment()?;
        let token_pair = TokenPair { from, to };
        let round_values = self
            .rounds()
            .get(&token_pair)
            .ok_or("token pair not found")?;
        let feed = self.make_price_feed(token_pair, round_values);
        Ok(MultiArg5::from((
            feed.round_id,
            feed.from,
            feed.to,
            feed.price,
            feed.decimals,
        )))
    }

    #[view(latestPriceFeedOptional)]
    fn latest_price_feed_optional(
        &self,
        from: BoxedBytes,
        to: BoxedBytes,
    ) -> OptionalResult<MultiArg5<u32, BoxedBytes, BoxedBytes, Self::BigUint, u8>> {
        self.latest_price_feed(from, to).ok().into()
    }

    #[endpoint(setSubmissionCount)]
    fn set_submission_count(&self, submission_count: u32) -> SCResult<()> {
        only_owner!(self, "Caller must be owner");
        self.submission_count().set(&submission_count);
        Ok(())
    }

    fn make_price_feed(
        &self,
        token_pair: TokenPair,
        round_values: VecMapper<Self::Storage, Self::BigUint>,
    ) -> PriceFeed<Self::BigUint> {
        let round_id = round_values.len();
        PriceFeed {
            round_id: round_id as u32,
            from: token_pair.from,
            to: token_pair.to,
            price: round_values.get(round_id),
            decimals: self.decimals().get(),
        }
    }

    fn subtract_query_payment(&self) -> SCResult<()> {
        self.transfer(
            self.blockchain().get_caller(),
            self.blockchain().get_sc_address(),
            &self.query_payment_amount().get(),
        )
    }

    #[view(getOracles)]
    fn get_oracles(&self) -> MultiResultVec<Address> {
        self.oracle_status().keys().collect()
    }

    #[view]
    #[storage_mapper("payment_token")]
    fn payment_token(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;

    #[view]
    #[storage_mapper("query_payment_amount")]
    fn query_payment_amount(&self) -> SingleValueMapper<Self::Storage, Self::BigUint>;

    #[view]
    #[storage_mapper("submission_count")]
    fn submission_count(&self) -> SingleValueMapper<Self::Storage, u32>;

    #[view]
    #[storage_mapper("decimals")]
    fn decimals(&self) -> SingleValueMapper<Self::Storage, u8>;

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
}
