#![no_std]

elrond_wasm::imports!();
pub mod median;

mod price_aggregator_data;
use price_aggregator_data::{OracleStatus, PriceFeed, TokenPair};

const SUBMISSION_LIST_MAX_LEN: usize = 50;

#[elrond_wasm::contract]
pub trait PriceAggregator {
    #[init]
    fn init(
        &self,
        submission_count: u32,
        decimals: u8,
        oracles: MultiValueEncoded<ManagedAddress>,
    ) {
        let is_deploy_call = !self.was_contract_deployed().get();
        if is_deploy_call {
            self.decimals().set(decimals);
            self.was_contract_deployed().set(true);
        }

        self.submission_count().set(submission_count);

        let mut oracle_mapper = self.oracle_status();
        for oracle in oracles {
            if !oracle_mapper.contains_key(&oracle) {
                oracle_mapper.insert(
                    oracle,
                    OracleStatus {
                        total_submissions: 0,
                        accepted_submissions: 0,
                    },
                );
            }
        }
    }

    #[endpoint]
    fn submit(&self, from: ManagedBuffer, to: ManagedBuffer, price: BigUint) {
        self.require_is_oracle();
        self.submit_unchecked(from, to, price);
    }

    fn submit_unchecked(&self, from: ManagedBuffer, to: ManagedBuffer, price: BigUint) {
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
        self.create_new_round(token_pair, submissions);
    }

    #[endpoint(submitBatch)]
    fn submit_batch(
        &self,
        submissions: MultiValueEncoded<MultiValue3<ManagedBuffer, ManagedBuffer, BigUint>>,
    ) {
        self.require_is_oracle();

        for (from, to, price) in submissions
            .into_iter()
            .map(|submission| submission.into_tuple())
        {
            self.submit_unchecked(from, to, price);
        }
    }

    fn require_is_oracle(&self) {
        require!(
            self.oracle_status()
                .contains_key(&self.blockchain().get_caller()),
            "only oracles allowed"
        );
    }

    fn create_new_round(
        &self,
        token_pair: TokenPair<Self::Api>,
        mut submissions: MapMapper<ManagedAddress, BigUint>,
    ) {
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
            let price_feed_result = median::calculate(submissions_vec.as_mut_slice());
            let price_feed_opt = price_feed_result.unwrap_or_else(|err| sc_panic!(err.as_bytes()));
            let price_feed = price_feed_opt.unwrap_or_else(|| sc_panic!("no submissions"));

            self.rounds()
                .entry(token_pair)
                .or_default()
                .get()
                .push(&price_feed);
            submissions.clear();
        }
    }

    #[view(latestRoundData)]
    fn latest_round_data(&self) -> MultiValueEncoded<PriceFeed<Self::Api>> {
        require!(!self.rounds().is_empty(), "no completed rounds");

        let mut result = MultiValueEncoded::new();
        for (token_pair, round_values) in self.rounds().iter() {
            result.push(self.make_price_feed(token_pair, round_values));
        }

        result
    }

    #[view(latestPriceFeed)]
    fn latest_price_feed(
        &self,
        from: ManagedBuffer,
        to: ManagedBuffer,
    ) -> SCResult<MultiValue5<u32, ManagedBuffer, ManagedBuffer, BigUint, u8>> {
        let token_pair = TokenPair { from, to };
        let round_values = self
            .rounds()
            .get(&token_pair)
            .ok_or("token pair not found")?;
        let feed = self.make_price_feed(token_pair, round_values);
        Ok((feed.round_id, feed.from, feed.to, feed.price, feed.decimals).into())
    }

    #[view(latestPriceFeedOptional)]
    fn latest_price_feed_optional(
        &self,
        from: ManagedBuffer,
        to: ManagedBuffer,
    ) -> OptionalValue<MultiValue5<u32, ManagedBuffer, ManagedBuffer, BigUint, u8>> {
        self.latest_price_feed(from, to).ok().into()
    }

    #[only_owner]
    #[endpoint(setSubmissionCount)]
    fn set_submission_count(&self, submission_count: u32) {
        self.submission_count().set(&submission_count);
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

    #[view(getOracles)]
    fn get_oracles(&self) -> MultiValueEncoded<ManagedAddress> {
        let mut result = MultiValueEncoded::new();
        for key in self.oracle_status().keys() {
            result.push(key);
        }
        result
    }

    #[storage_mapper("was_contract_deployed")]
    fn was_contract_deployed(&self) -> SingleValueMapper<bool>;

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
}
