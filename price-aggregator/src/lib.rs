#![no_std]
#![feature(destructuring_assignment)]

elrond_wasm::imports!();
pub mod median;

mod price_aggregator_data;
use arrayvec::ArrayVec;
use price_aggregator_data::{OracleStatus, PriceFeed, TokenPair};

const SUBMISSION_LIST_MAX_LEN: usize = 50;

#[elrond_wasm::derive::contract]
pub trait PriceAggregator {
    #[init]
    fn init(
        &self,
        payment_token: TokenIdentifier,
        oracles: ManagedVec<ManagedAddress>,
        submission_count: u32,
        decimals: u8,
        query_payment_amount: BigUint,
    ) -> SCResult<()> {
        self.payment_token().set(&payment_token);
        self.query_payment_amount().set(&query_payment_amount);
        self.submission_count().set(&submission_count);
        self.decimals().set(&decimals);
        for oracle in oracles.iter() {
            self.oracle_status().insert(
                oracle.clone(),
                OracleStatus {
                    total_submissions: 0,
                    accepted_submissions: 0,
                },
            );
        }
        Ok(())
    }

    #[endpoint]
    #[payable("*")]
    fn deposit(
        &self,
        #[payment] payment: BigUint,
        #[payment_token] token: TokenIdentifier,
        #[var_args] on_behalf_of: OptionalArg<ManagedAddress>,
    ) -> SCResult<()> {
        require!(token == self.payment_token().get(), "wrong token type");
        let to = on_behalf_of
            .into_option()
            .unwrap_or_else(|| self.blockchain().get_caller());
        self.add_balance(to, &payment);
        Ok(())
    }

    fn add_balance(&self, to: ManagedAddress, amount: &BigUint) {
        self.balance()
            .entry(to)
            .or_insert_with(|| BigUint::zero())
            .update(|balance| *balance += amount.clone());
    }

    fn subtract_balance(&self, from: ManagedAddress, amount: &BigUint) -> SCResult<()> {
        self.balance()
            .entry(from)
            .or_insert_with(|| BigUint::zero())
            .update(|balance| {
                require!(*balance >= *amount, "insufficient balance");
                *balance -= amount.clone();
                Ok(())
            })
    }

    #[endpoint]
    fn withdraw(&self, amount: BigUint) -> SCResult<()> {
        let caller = self.blockchain().get_caller();
        self.subtract_balance(caller.clone(), &amount)?;
        self.send()
            .direct(&caller, &self.payment_token().get(), 0, &amount, &[]);
        Ok(())
    }

    fn transfer(&self, from: ManagedAddress, to: ManagedAddress, amount: &BigUint) -> SCResult<()> {
        self.subtract_balance(from, amount)?;
        self.add_balance(to, amount);
        Ok(())
    }

    #[endpoint]
    fn submit(&self, from: ManagedBuffer, to: ManagedBuffer, price: BigUint) -> SCResult<()> {
        self.require_is_oracle()?;
        self.submit_unchecked(from, to, price)
    }

    fn submit_unchecked(
        &self,
        from: ManagedBuffer,
        to: ManagedBuffer,
        price: BigUint,
    ) -> SCResult<()> {
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

    #[endpoint(submitBatch)]
    fn submit_batch(
        &self,
        #[var_args] submissions: ManagedVarArgs<MultiArg3<ManagedBuffer, ManagedBuffer, BigUint>>,
    ) -> SCResult<()> {
        self.require_is_oracle()?;

        for (from, to, price) in submissions
            .into_iter()
            .map(|submission| submission.into_tuple())
        {
            self.submit_unchecked(from, to, price)?;
        }
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
        token_pair: TokenPair<Self::Api>,
        mut submissions: MapMapper<ManagedAddress, BigUint>,
    ) -> SCResult<()> {
        let submissions_len = submissions.len();
        if submissions_len as u32 >= self.submission_count().get() {
            require!(
                submissions_len <= SUBMISSION_LIST_MAX_LEN,
                "submission list capacity exceeded"
            );
            let mut submissions_vec = ArrayVec::<BigUint, SUBMISSION_LIST_MAX_LEN>::new();
            for submission_value in submissions.values() {
                submissions_vec.push(submission_value);
            }
            let price_feed =
                median::calculate(submissions_vec.as_mut_slice())?.ok_or("no submissions")?;
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
    fn my_balance(&self) -> BigUint {
        self.get_balance(self.blockchain().get_caller())
    }

    #[view(getBalance)]
    fn get_balance(&self, address: ManagedAddress) -> BigUint {
        self.balance()
            .get(&address)
            .unwrap_or_else(|| BigUint::zero())
    }

    #[view(latestRoundData)]
    fn latest_round_data(&self) -> SCResult<ManagedMultiResultVec<PriceFeed<Self::Api>>> {
        self.subtract_query_payment()?;
        require!(!self.rounds().is_empty(), "no completed rounds");
        let mut result = ManagedMultiResultVec::new();
        for (token_pair, round_values) in self.rounds().iter() {
            result.push(self.make_price_feed(token_pair, round_values));
        }
        Ok(result)
    }

    #[endpoint(latestPriceFeed)]
    fn latest_price_feed(
        &self,
        from: ManagedBuffer,
        to: ManagedBuffer,
    ) -> SCResult<MultiArg5<u32, ManagedBuffer, ManagedBuffer, BigUint, u8>> {
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
        from: ManagedBuffer,
        to: ManagedBuffer,
    ) -> OptionalResult<MultiArg5<u32, ManagedBuffer, ManagedBuffer, BigUint, u8>> {
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
        token_pair: TokenPair<Self::Api>,
        round_values: VecMapper<BigUint>,
    ) -> PriceFeed<Self::Api> {
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
    fn get_oracles(&self) -> ManagedMultiResultVec<ManagedAddress> {
        let mut result = ManagedMultiResultVec::new();
        for key in self.oracle_status().keys() {
            result.push(key);
        }
        result
    }

    #[view]
    #[storage_mapper("payment_token")]
    fn payment_token(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view]
    #[storage_mapper("query_payment_amount")]
    fn query_payment_amount(&self) -> SingleValueMapper<BigUint>;

    #[view]
    #[storage_mapper("submission_count")]
    fn submission_count(&self) -> SingleValueMapper<u32>;

    #[view]
    #[storage_mapper("decimals")]
    fn decimals(&self) -> SingleValueMapper<u8>;

    #[storage_mapper("oracle_status")]
    fn oracle_status(&self) -> MapMapper<ManagedAddress, OracleStatus>;

    #[storage_mapper("rounds")]
    fn rounds(&self) -> MapStorageMapper<TokenPair<Self::Api>, VecMapper<BigUint>>;

    #[storage_mapper("submissions")]
    fn submissions(
        &self,
    ) -> MapStorageMapper<TokenPair<Self::Api>, MapMapper<ManagedAddress, BigUint>>;

    #[storage_mapper("balance")]
    fn balance(&self) -> MapMapper<ManagedAddress, BigUint>;
}
