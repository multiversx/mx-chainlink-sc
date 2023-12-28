#![no_std]

multiversx_sc::imports!();
mod aggregator_data;
pub mod aggregator_interface;
pub mod median;

use core::borrow::Borrow;

use aggregator_data::{
    AddressAmountPair, Funds, OracleRoundState, OracleStatus, Requester, RoundDetails, Submission,
};
use aggregator_interface::{DescriptionVec, Round, SingleSubmissionValuesVec};

const RESERVE_ROUNDS: u64 = 2;
const ROUND_MAX: u64 = u64::MAX;

#[multiversx_sc::contract]
pub trait Aggregator {
    #[storage_mapper("token_id")]
    fn token_id(&self) -> SingleValueMapper<EgldOrEsdtTokenIdentifier>;

    // Round related params
    #[storage_mapper("payment_amount")]
    fn payment_amount(&self) -> SingleValueMapper<BigUint>;

    #[storage_mapper("max_submission_count")]
    fn max_submission_count(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("min_submission_count")]
    fn min_submission_count(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("restart_delay")]
    fn restart_delay(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("timeout")]
    fn timeout(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("min_submission_value")]
    fn min_submission_value(&self) -> SingleValueMapper<BigUint>;

    #[storage_mapper("max_submission_value")]
    fn max_submission_value(&self) -> SingleValueMapper<BigUint>;

    #[storage_mapper("reporting_round_id")]
    fn reporting_round_id(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("latest_round_id")]
    fn latest_round_id(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("oracles")]
    fn oracles(&self) -> MapMapper<ManagedAddress, OracleStatus<Self::Api>>;

    #[storage_mapper("rounds")]
    fn rounds(&self) -> MapMapper<u64, Round<Self::Api>>;

    #[storage_mapper("details")]
    fn details(&self) -> MapMapper<u64, RoundDetails<Self::Api>>;

    #[storage_mapper("requesters")]
    fn requesters(&self) -> MapMapper<ManagedAddress, Requester>;

    #[storage_mapper("recorded_funds")]
    fn recorded_funds(&self) -> SingleValueMapper<Funds<Self::Api>>;

    #[storage_mapper("deposits")]
    fn deposits(&self) -> MapMapper<ManagedAddress, BigUint>;

    #[storage_mapper("decimals")]
    fn decimals(&self) -> SingleValueMapper<u8>;

    #[storage_mapper("description")]
    fn description(&self) -> SingleValueMapper<DescriptionVec>;

    #[storage_mapper("values_count")]
    fn values_count(&self) -> SingleValueMapper<usize>;

    #[init]
    fn init(
        &self,
        token_id: EgldOrEsdtTokenIdentifier,
        payment_amount: BigUint,
        timeout: u64,
        min_submission_value: BigUint,
        max_submission_value: BigUint,
        decimals: u8,
        description: DescriptionVec,
        values_count: usize,
    ) {
        self.token_id().set(&token_id);
        self.recorded_funds().set(&Funds {
            available: BigUint::zero(),
            allocated: BigUint::zero(),
        });

        self.update_future_rounds_internal(payment_amount, 0, 0, 0, timeout);
        self.min_submission_value().set(&min_submission_value);
        self.max_submission_value().set(&max_submission_value);
        self.decimals().set(&decimals);
        self.description().set(&description);
        self.values_count().set(&values_count);
        self.initialize_new_round(0);
    }

    #[endpoint(addFunds)]
    #[payable("*")]
    fn add_funds(&self) {
        let (token, payment) = self.call_value().egld_or_single_fungible_esdt();
        require!(token == self.token_id().get(), "Wrong token type");
        self.recorded_funds()
            .update(|recorded_funds| recorded_funds.available += &payment);
        let caller = &self.blockchain().get_caller();
        let deposit = self.get_deposit(caller) + payment;
        self.set_deposit(caller, &deposit);
    }

    fn get_deposit(&self, address: &ManagedAddress) -> BigUint {
        self.deposits()
            .get(address)
            .unwrap_or_else(|| BigUint::zero())
    }

    fn set_deposit(&self, address: &ManagedAddress, amount: &BigUint) {
        if amount == &BigUint::zero() {
            self.deposits().remove(address);
        } else {
            self.deposits().insert(address.clone(), amount.clone());
        }
    }

    fn validate_submission_limits(&self, submission_values: &SingleSubmissionValuesVec<Self::Api>) {
        for value in submission_values.iter() {
            require!(
                value >= &self.min_submission_value().get(),
                "value below min_submission_value"
            );
            require!(
                value <= &self.max_submission_value().get(),
                "value above max_submission_value"
            );
        }
    }

    #[endpoint(submit)]
    fn submit(&self, round_id: u64, submission_values: MultiValueEncoded<BigUint>) {
        require!(
            submission_values.len() == self.values_count().get(),
            "incorrect number of values in submission"
        );
        if let SCResult::Err(err) =
            self.validate_oracle_round(&self.blockchain().get_caller(), round_id)
        {
            sc_panic!(err.as_bytes())
        }

        let mut values = ArrayVec::new();
        for val in submission_values {
            values.push(val);
        }

        self.validate_submission_limits(&values);
        self.oracle_initialize_new_round(round_id);
        self.record_submission(Submission { values }, round_id);
        self.update_round_answer(round_id);
        self.pay_oracle(round_id);
        self.delete_round_details(round_id);
    }

    #[only_owner]
    #[endpoint(changeOracles)]
    fn change_oracles(
        &self,
        removed: ManagedVec<ManagedAddress>,
        added: ManagedVec<ManagedAddress>,
        added_admins: ManagedVec<ManagedAddress>,
        min_submissions: u64,
        max_submissions: u64,
        restart_delay: u64,
    ) {
        for oracle in &removed {
            self.oracles().remove(&oracle);
        }

        require!(
            added.len() == added_admins.len(),
            "need same oracle and admin count"
        );

        for (added_oracle, added_admin) in added.iter().zip(added_admins.iter()) {
            self.add_oracle(added_oracle.borrow(), added_admin.borrow());
        }

        self.update_future_rounds_internal(
            self.payment_amount().get(),
            min_submissions,
            max_submissions,
            restart_delay,
            self.timeout().get(),
        );
    }

    #[only_owner]
    #[endpoint(updateFutureRounds)]
    fn update_future_rounds(
        &self,
        payment_amount: BigUint,
        min_submissions: u64,
        max_submissions: u64,
        restart_delay: u64,
        timeout: u64,
    ) {
        self.update_future_rounds_internal(
            payment_amount,
            min_submissions,
            max_submissions,
            restart_delay,
            timeout,
        )
    }

    fn update_future_rounds_internal(
        &self,
        payment_amount: BigUint,
        min_submissions: u64,
        max_submissions: u64,
        restart_delay: u64,
        timeout: u64,
    ) {
        let oracle_count = self.oracle_count();
        require!(
            max_submissions >= min_submissions,
            "max must equal/exceed min"
        );
        require!(max_submissions <= oracle_count, "max cannot exceed total");
        require!(
            oracle_count == 0 || restart_delay < oracle_count,
            "delay cannot exceed total"
        );

        let recorded_funds = self.recorded_funds().get();

        require!(
            recorded_funds.available >= self.required_reserve(&payment_amount),
            "insufficient funds for payment"
        );

        if oracle_count > 0 {
            require!(min_submissions > 0, "min must be greater than 0");
        }
        self.payment_amount().set(&payment_amount);
        self.min_submission_count().set(&min_submissions);
        self.max_submission_count().set(&max_submissions);
        self.restart_delay().set(&restart_delay);
        self.timeout().set(&timeout);
    }

    #[view(allocatedFunds)]
    fn allocated_funds(&self) -> BigUint {
        self.recorded_funds().get().allocated
    }

    #[view(availableFunds)]
    fn available_funds(&self) -> BigUint {
        self.recorded_funds().get().available
    }

    #[view(oracleCount)]
    fn oracle_count(&self) -> u64 {
        self.oracles().len() as u64
    }

    #[view(getRoundData)]
    fn get_round_data(&self, round_id: u64) -> OptionalValue<Round<Self::Api>> {
        self.rounds().get(&round_id).into()
    }

    #[view(latestRoundData)]
    fn latest_round_data(&self) -> OptionalValue<Round<Self::Api>> {
        self.get_round_data(self.latest_round_id().get())
    }

    #[view(withdrawablePayment)]
    fn withdrawable_payment(&self, oracle: ManagedAddress) -> BigUint {
        self.get_oracle_status_result(&oracle).withdrawable
    }

    #[endpoint(withdrawPayment)]
    fn withdraw_payment(&self, oracle: ManagedAddress, recipient: ManagedAddress, amount: BigUint) {
        let mut oracle_status = self.get_oracle_status_result(&oracle);
        require!(
            oracle_status.admin == self.blockchain().get_caller(),
            "only callable by admin"
        );

        require!(
            oracle_status.withdrawable >= amount,
            "insufficient withdrawable funds"
        );

        self.recorded_funds()
            .update(|recorded_funds| recorded_funds.allocated -= &amount);
        oracle_status.withdrawable -= &amount;
        self.oracles().insert(oracle, oracle_status);

        self.send()
            .direct(&recipient, &self.token_id().get(), 0, &amount);
    }

    #[view(withdrawableAddedFunds)]
    fn withdrawable_added_funds(&self) -> BigUint {
        self.get_deposit(&self.blockchain().get_caller())
    }

    #[endpoint(withdrawFunds)]
    fn withdraw_funds(&self, amount: BigUint) {
        let recorded_funds = self.recorded_funds().get();
        let caller = &self.blockchain().get_caller();
        let deposit = self.get_deposit(caller);
        require!(amount <= deposit, "Insufficient funds to withdraw");
        require!(
            recorded_funds.available - self.required_reserve(&self.payment_amount().get())
                >= amount,
            "insufficient reserve funds"
        );
        self.recorded_funds()
            .update(|recorded_funds| recorded_funds.available -= &amount);
        let remaining = &deposit - &amount;
        self.set_deposit(caller, &remaining);
        self.send()
            .direct(caller, &self.token_id().get(), 0, &amount);
    }

    #[view(getAdmin)]
    fn get_admin(&self, oracle: ManagedAddress) -> ManagedAddress {
        self.get_oracle_status_result(&oracle).admin
    }

    #[endpoint(transferAdmin)]
    fn transfer_admin(&self, oracle: ManagedAddress, new_admin: ManagedAddress) {
        let mut oracle_status = self.get_oracle_status_result(&oracle);
        require!(
            oracle_status.admin == self.blockchain().get_caller(),
            "only callable by admin"
        );
        oracle_status.pending_admin = Some(new_admin);
        self.oracles().insert(oracle, oracle_status);
    }

    #[endpoint(acceptAdmin)]
    fn accept_admin(&self, oracle: ManagedAddress) {
        let mut oracle_status = self.get_oracle_status_result(&oracle);
        let caller = self.blockchain().get_caller();
        require!(
            oracle_status.pending_admin == Some(caller.clone()),
            "only callable by pending admin"
        );
        oracle_status.pending_admin = None;
        oracle_status.admin = caller;
        self.oracles().insert(oracle, oracle_status);
    }

    #[endpoint(requestNewRound)]
    fn request_new_round(&self) -> u64 {
        let requester_option = self.requesters().get(&self.blockchain().get_caller());
        require!(
            requester_option.map_or_else(|| false, |requester| requester.authorized),
            "not authorized requester"
        );

        let current = self.reporting_round_id().get();
        require!(
            self.rounds()
                .get(&current)
                .map_or_else(|| false, |round| round.updated_at > 0)
                || self.timed_out(current),
            "prev round must be supersedable"
        );

        let new_round_id = current + 1;
        self.requester_initialize_new_round(new_round_id);

        new_round_id
    }

    #[only_owner]
    #[endpoint(setRequesterPermissions)]
    fn set_requester_permissions(&self, requester: ManagedAddress, authorized: bool, delay: u64) {
        if authorized {
            self.requesters().insert(
                requester,
                Requester {
                    authorized,
                    delay,
                    last_started_round: 0,
                },
            );
        } else {
            self.requesters().remove(&requester);
        }
    }

    #[view(oracleRoundState)]
    fn oracle_round_state(
        &self,
        oracle: ManagedAddress,
        queried_round_id: u64,
    ) -> OracleRoundState<Self::Api> {
        if queried_round_id == 0 {
            return self.oracle_round_state_suggest_round(&oracle);
        }
        let eligible_to_submit = self.eligible_for_specific_round(&oracle, queried_round_id);
        let round = self.get_round(queried_round_id);
        let details = self.get_round_details(queried_round_id);
        let oracle_status = self.get_oracle_status_result(&oracle);
        let recorded_funds = self.recorded_funds().get();

        OracleRoundState {
            eligible_to_submit,
            round_id: queried_round_id,
            latest_submission: oracle_status.latest_submission,
            started_at: round.started_at,
            timeout: details.timeout,
            available_funds: recorded_funds.available,
            oracle_count: self.oracle_count(),
            payment_amount: if round.started_at > 0 {
                details.payment_amount
            } else {
                self.payment_amount().get()
            },
        }
    }

    fn initialize_new_round(&self, round_id: u64) {
        if let Some(last_round) = round_id.checked_sub(1) {
            self.update_timed_out_round_info(last_round);
        }

        self.reporting_round_id().set(round_id);
        self.rounds().insert(
            round_id.clone(),
            Round {
                round_id: round_id.clone(),
                answer: None,
                decimals: self.decimals().get(),
                description: self.description().get(),
                started_at: self.blockchain().get_block_timestamp(),
                updated_at: self.blockchain().get_block_timestamp(),
                answered_in_round: 0,
            },
        );
        self.details().insert(
            round_id.clone(),
            RoundDetails {
                submissions: ArrayVec::new(),
                max_submissions: self.max_submission_count().get(),
                min_submissions: self.min_submission_count().get(),
                timeout: self.timeout().get(),
                payment_amount: self.payment_amount().get(),
            },
        );
    }

    fn oracle_initialize_new_round(&self, round_id: u64) {
        if !self.new_round(round_id) {
            return;
        }
        let oracle = self.blockchain().get_caller();
        let mut oracle_status = self.get_oracle_status_result(&oracle);
        let restart_delay = self.restart_delay().get();
        if round_id <= oracle_status.last_started_round + restart_delay
            && oracle_status.last_started_round != 0
        {
            return;
        }

        self.initialize_new_round(round_id);

        oracle_status.last_started_round = round_id;
        self.oracles().insert(oracle, oracle_status);
    }

    fn requester_initialize_new_round(&self, round_id: u64) {
        let requester_address = self.blockchain().get_caller();
        let mut requester = self.get_requester(&requester_address);

        if !self.new_round(round_id) {
            return;
        }

        require!(
            round_id > requester.last_started_round + requester.delay
                || requester.last_started_round == 0,
            "must delay requests"
        );

        self.initialize_new_round(round_id);

        requester.last_started_round = round_id;
        self.requesters().insert(requester_address, requester);
    }

    fn update_timed_out_round_info(&self, round_id: u64) {
        if !self.timed_out(round_id) {
            return;
        }
        let mut round = self.get_round(round_id);
        if let Some(prev_id) = round_id.checked_sub(1) {
            let prev_round = self.get_round(prev_id);
            round.answer = prev_round.answer;
            round.answered_in_round = prev_round.answered_in_round;
        } else {
            round.answer = None;
            round.answered_in_round = 0;
        }
        round.updated_at = self.blockchain().get_block_timestamp();
        self.rounds().insert(round_id, round);
        self.details().remove(&round_id);
    }

    fn eligible_for_specific_round(&self, oracle: &ManagedAddress, queried_round_id: u64) -> bool {
        if self
            .rounds()
            .get(&queried_round_id)
            .map_or_else(|| false, |round| round.started_at > 0)
        {
            self.accepting_submissions(queried_round_id)
                && self.validate_oracle_round(oracle, queried_round_id).is_ok()
        } else {
            self.delayed(oracle, queried_round_id)
                && self.validate_oracle_round(oracle, queried_round_id).is_ok()
        }
    }

    fn oracle_round_state_suggest_round(
        &self,
        oracle: &ManagedAddress,
    ) -> OracleRoundState<Self::Api> {
        let oracle_status = self.get_oracle_status_result(oracle);

        let reporting_round_id = self.reporting_round_id().get();
        let should_supersede = oracle_status.last_reported_round == reporting_round_id
            || !self.accepting_submissions(reporting_round_id);
        // Instead of nudging oracles to submit to the next round, the inclusion of
        // the should_supersede bool in the if condition pushes them towards
        // submitting in a currently open round.
        let mut eligible_to_submit: bool;
        let round: Round<Self::Api>;
        let round_id: u64;
        let payment_amount: BigUint;
        if self.supersedable(reporting_round_id) && should_supersede {
            round_id = reporting_round_id + 1;
            round = self.get_round(round_id);

            payment_amount = self.payment_amount().get();
            eligible_to_submit = self.delayed(&oracle, round_id);
        } else {
            round_id = reporting_round_id;
            round = self.get_round(round_id);

            let round_details = self.get_round_details(round_id);
            payment_amount = round_details.payment_amount;
            eligible_to_submit = self.accepting_submissions(round_id);
        }

        if self.validate_oracle_round(&oracle, round_id).is_err() {
            eligible_to_submit = false;
        }

        let recorded_funds = self.recorded_funds().get();
        let round_details = self.get_round_details(round_id);

        OracleRoundState {
            eligible_to_submit,
            round_id,
            latest_submission: oracle_status.latest_submission,
            started_at: round.started_at,
            timeout: round_details.timeout,
            available_funds: recorded_funds.available,
            oracle_count: self.oracle_count(),
            payment_amount,
        }
    }

    fn update_round_answer(&self, round_id: u64) {
        let details = self.get_round_details(round_id);
        if (details.submissions.len() as u64) < details.min_submissions {
            return;
        }

        match median::calculate_submission_median(details.submissions) {
            Result::Ok(new_answer) => {
                let mut round = self.get_round(round_id);
                round.answer = new_answer;
                round.updated_at = self.blockchain().get_block_timestamp();
                round.answered_in_round = round_id;
                self.rounds().insert(round_id, round);
                self.latest_round_id().set(&round_id);
            }
            Result::Err(error_message) => sc_panic!(error_message.as_bytes()),
        }
    }

    fn subtract_amount_from_deposits(&self, amount: &BigUint) {
        let mut remaining = amount.clone();
        let mut final_amounts = ManagedVec::<Self::Api, AddressAmountPair<Self::Api>>::new();
        for (account, deposit) in self.deposits().iter() {
            if remaining == BigUint::zero() {
                break;
            }
            if deposit <= remaining {
                final_amounts.push(AddressAmountPair {
                    address: account,
                    amount: BigUint::zero(),
                });
                remaining -= deposit;
            } else {
                final_amounts.push(AddressAmountPair {
                    address: account,
                    amount: deposit - remaining,
                });
                remaining = BigUint::zero();
            }
        }
        for pair in &final_amounts {
            self.set_deposit(&pair.address, &pair.amount);
        }
    }

    fn pay_oracle(&self, round_id: u64) {
        let round_details = self.get_round_details(round_id);
        let oracle = self.blockchain().get_caller();
        let mut oracle_status = self.get_oracle_status_result(&oracle);

        let payment = round_details.payment_amount;
        self.recorded_funds().update(|recorded_funds| {
            recorded_funds.available -= &payment;
            recorded_funds.allocated += &payment;
        });
        self.subtract_amount_from_deposits(&payment);

        oracle_status.withdrawable += &payment;
        self.oracles().insert(oracle, oracle_status);
    }

    fn record_submission(&self, submission: Submission<Self::Api>, round_id: u64) {
        require!(
            self.accepting_submissions(round_id),
            "round not accepting submissions"
        );

        let mut round_details = self.get_round_details(round_id);
        let oracle = self.blockchain().get_caller();
        let mut oracle_status = self.get_oracle_status_result(&oracle);
        round_details.submissions.push(submission.clone());
        oracle_status.last_reported_round = round_id;
        oracle_status.latest_submission = Some(submission);
        self.details().insert(round_id, round_details);
        self.oracles().insert(oracle, oracle_status);
    }

    fn delete_round_details(&self, round_id: u64) {
        if let Some(details) = self.details().get(&round_id) {
            if (details.submissions.len() as u64) < details.max_submissions {
                return;
            }
        }
        self.details().remove(&round_id);
    }

    fn timed_out(&self, round_id: u64) -> bool {
        let round = self.get_round(round_id);
        let started_at = round.started_at;
        let details = self.get_round_details(round_id);
        let round_timeout = details.timeout;

        round_id == 0
            || (started_at > 0
                && round_timeout > 0
                && started_at + round_timeout < self.blockchain().get_block_timestamp())
    }

    fn get_starting_round(&self, oracle: &ManagedAddress) -> u64 {
        let current_round = self.reporting_round_id().get();
        if current_round != 0 {
            if let Some(oracle_status) = self.get_oracle_status_option(&oracle) {
                if current_round == oracle_status.ending_round {
                    return current_round;
                }
            }
        }
        current_round + 1
    }

    fn previous_and_current_unanswered(&self, round_id: u64, rr_id: u64) -> bool {
        let round = self.get_round(rr_id);
        round_id + 1 == rr_id && round.updated_at == 0
    }

    #[view(requiredReserve)]
    fn required_reserve(&self, payment: &BigUint) -> BigUint {
        payment * &BigUint::from(self.oracle_count()) * BigUint::from(RESERVE_ROUNDS)
    }

    fn add_oracle(&self, oracle: &ManagedAddress, admin: &ManagedAddress) {
        require!(!self.oracle_enabled(oracle), "oracle already enabled");

        self.oracles().insert(
            oracle.clone(),
            OracleStatus {
                withdrawable: BigUint::zero(),
                starting_round: self.get_starting_round(oracle),
                ending_round: ROUND_MAX,
                last_reported_round: 0,
                last_started_round: 0,
                latest_submission: None,
                admin: admin.clone(),
                pending_admin: None,
            },
        );
    }

    fn validate_oracle_round(&self, oracle: &ManagedAddress, round_id: u64) -> SCResult<()> {
        let oracle_status = self.get_oracle_status_result(&oracle);
        let reporting_round_id = self.reporting_round_id().get();

        require_old!(oracle_status.starting_round != 0, "not enabled oracle");
        require_old!(
            oracle_status.starting_round <= round_id,
            "not yet enabled oracle"
        );
        require_old!(
            oracle_status.ending_round >= round_id,
            "no longer allowed oracle"
        );
        require_old!(
            oracle_status.last_reported_round < round_id,
            "cannot report on previous rounds"
        );
        require_old!(
            round_id == reporting_round_id
                || round_id == reporting_round_id + 1
                || self.previous_and_current_unanswered(round_id, reporting_round_id),
            "invalid round to report"
        );
        require_old!(
            round_id == 1 || self.supersedable(round_id - 1),
            "previous round not supersedable"
        );

        Ok(())
    }

    fn supersedable(&self, round_id: u64) -> bool {
        let round = self.get_round(round_id);
        let timed_out = self.timed_out(round_id);
        round.updated_at > 0 || timed_out
    }

    fn oracle_enabled(&self, oracle: &ManagedAddress) -> bool {
        self.oracles().contains_key(oracle)
    }

    fn accepting_submissions(&self, round_id: u64) -> bool {
        let details = self.get_round_details(round_id);
        details.max_submissions != 0
    }

    fn delayed(&self, oracle: &ManagedAddress, round_id: u64) -> bool {
        let oracle_status = self.get_oracle_status_result(oracle);
        let last_started = oracle_status.last_started_round;
        round_id > last_started + self.restart_delay().get() || last_started == 0
    }

    fn new_round(&self, round_id: u64) -> bool {
        round_id == self.reporting_round_id().get() + 1
    }

    fn get_oracle_status_option(&self, oracle: &ManagedAddress) -> Option<OracleStatus<Self::Api>> {
        self.oracles().get(oracle)
    }

    fn get_oracle_status_result(&self, oracle: &ManagedAddress) -> OracleStatus<Self::Api> {
        self.oracles()
            .get(oracle)
            .unwrap_or_else(|| sc_panic!("No oracle at given address"))
    }

    fn get_round(&self, round_id: u64) -> Round<Self::Api> {
        self.rounds()
            .get(&round_id)
            .unwrap_or_else(|| sc_panic!("No round for given round id"))
    }

    fn get_round_details(&self, round_id: u64) -> RoundDetails<Self::Api> {
        self.details()
            .get(&round_id)
            .unwrap_or_else(|| sc_panic!("No round details for given round id"))
    }

    fn get_requester(&self, requester_address: &ManagedAddress) -> Requester {
        self.requesters()
            .get(requester_address)
            .unwrap_or_else(|| sc_panic!("No requester has the given address"))
    }

    #[view(getOracles)]
    fn get_oracles(&self) -> MultiValueEncoded<ManagedAddress> {
        let mut oracles = MultiValueEncoded::new();
        for oracle in self.oracles().keys() {
            oracles.push(oracle);
        }

        oracles
    }
}
