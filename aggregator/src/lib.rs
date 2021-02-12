#![no_std]

elrond_wasm::imports!();
use elrond_wasm::{
    api::{ErrorApi, StorageReadApi, StorageWriteApi},
    String,
};
mod aggregator_data;
use aggregator_data::{Funds, OracleRoundState, OracleStatus, Requester, Round, RoundDetails};
mod aggregator_interface;
use aggregator_interface::RoundData;
use elrond_wasm::elrond_codec;

const VERSION: u64 = 3;

const RESERVE_ROUNDS: u64 = 2;
const ROUND_MAX: u64 = u64::MAX;

fn set_value<Storage, Value>(mut storage: SingleValueMapper<Storage, Value>, value: Value)
where
    Storage: StorageReadApi + StorageWriteApi + ErrorApi + Clone,
    Value: elrond_codec::TopEncode + elrond_codec::TopDecode + 'static,
{
    storage.value = value;
    storage.save();
}

fn median(mut numbers: Vec<u64>) -> u64 {
    numbers.sort();
    let mid = numbers.len() / 2;
    numbers[mid]
}

#[elrond_wasm_derive::contract(AggregatorImpl)]
pub trait Aggregator {
    #[storage_mapper("link_token")]
    fn link_token(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;

    //AggregatorValidatorInterface public validator;
    #[storage_mapper("validator")]
    fn validator(&self) -> SingleValueMapper<Self::Storage, Address>;

    // Round related params
    #[storage_mapper("payment_amount")]
    fn payment_amount(&self) -> SingleValueMapper<Self::Storage, BigUint>;

    #[storage_mapper("max_submission_count")]
    fn max_submission_count(&self) -> SingleValueMapper<Self::Storage, u64>;

    #[storage_mapper("min_submission_count")]
    fn min_submission_count(&self) -> SingleValueMapper<Self::Storage, u64>;

    #[storage_mapper("restart_delay")]
    fn restart_delay(&self) -> SingleValueMapper<Self::Storage, u64>;

    #[storage_mapper("timeout")]
    fn timeout(&self) -> SingleValueMapper<Self::Storage, u64>;

    #[storage_mapper("min_submission_value")]
    fn min_submission_value(&self) -> SingleValueMapper<Self::Storage, u64>;

    #[storage_mapper("max_submission_value")]
    fn max_submission_value(&self) -> SingleValueMapper<Self::Storage, u64>;

    #[storage_mapper("reporting_round_id")]
    fn reporting_round_id(&self) -> SingleValueMapper<Self::Storage, u64>;

    #[storage_mapper("latest_round_id")]
    fn latest_round_id(&self) -> SingleValueMapper<Self::Storage, u64>;

    #[storage_mapper("oracles")]
    fn oracle_addresses(&self) -> SetMapper<Self::Storage, Address>;

    #[storage_mapper("oracles")]
    fn oracles(&self) -> MapMapper<Self::Storage, Address, OracleStatus<BigUint>>;

    #[storage_mapper("rounds")]
    fn rounds(&self) -> MapMapper<Self::Storage, u64, Round>;

    #[storage_mapper("details")]
    fn details(&self) -> MapMapper<Self::Storage, u64, RoundDetails<BigUint>>;

    #[storage_mapper("requesters")]
    fn requesters(&self) -> MapMapper<Self::Storage, Address, Requester>;

    #[storage_mapper("recorded_funds")]
    fn recorded_funds(&self) -> SingleValueMapper<Self::Storage, Funds<BigUint>>;

    #[storage_mapper("decimals")]
    fn decimals(&self) -> SingleValueMapper<Self::Storage, u8>;

    #[storage_mapper("description")]
    fn description(&self) -> SingleValueMapper<Self::Storage, String>;

    #[storage_mapper("version")]
    fn version(&self) -> SingleValueMapper<Self::Storage, u64>;

    #[init]
    fn init(
        &self,
        link_token: TokenIdentifier,
        payment_amount: BigUint,
        timeout: u64,
        validator: Address,
        min_submission_value: u64,
        max_submission_value: u64,
        decimals: u8,
        description: String,
    ) -> SCResult<()> {
        set_value(self.version(), VERSION);
        set_value(self.link_token(), link_token);

        sc_try!(self.update_future_rounds(payment_amount, 0, 0, 0, timeout));
        set_value(self.validator(), validator);
        set_value(self.min_submission_value(), min_submission_value);
        set_value(self.max_submission_value(), max_submission_value);
        set_value(self.decimals(), decimals);
        set_value(self.description(), description);

        //rounds[0].updatedAt = uint64(block.timestamp.sub(uint256(_timeout)));
        self.rounds().insert(
            0,
            Round {
                answer: 0,
                started_at: 0,
                updated_at: self.get_block_timestamp() - timeout,
                answered_in_round: 0,
            },
        );

        Ok(())
    }

    fn submit(&self, round_id: u64, submission: u64) -> SCResult<()> {
        sc_try!(self.validate_oracle_round(&self.get_caller(), &round_id));
        require!(
            submission >= self.min_submission_value().value,
            "value below min_submission_value"
        );
        require!(
            submission <= self.max_submission_value().value,
            "value above max_submission_value"
        );

        sc_try!(self.oracle_initialize_new_round(round_id));
        sc_try!(self.record_submission(submission, round_id));
        sc_try!(self.update_round_answer(round_id));
        sc_try!(self.pay_oracle(round_id));
        self.delete_round_details(round_id);
        Ok(())
    }

    fn change_oracles(
        &self,
        removed: Vec<Address>,
        added: Vec<Address>,
        added_admins: Vec<Address>,
        min_submissions: u64,
        max_submissions: u64,
        restart_delay: u64,
    ) -> SCResult<()> {
        only_owner!(self, "Only owner may call this function!");
        for oracle in removed.iter() {
            self.oracles().remove(oracle);
        }

        require!(
            added.len() == added_admins.len(),
            "need same oracle and admin count"
        );

        for (added_oracle, added_admin) in added.iter().zip(added_admins.iter()) {
            sc_try!(self.add_oracle(added_oracle, added_admin));
        }

        sc_try!(self.update_future_rounds(
            self.payment_amount().value,
            min_submissions,
            max_submissions,
            restart_delay,
            self.timeout().value,
        ));
        Ok(())
    }

    fn update_future_rounds(
        &self,
        payment_amount: BigUint,
        min_submissions: u64,
        max_submissions: u64,
        restart_delay: u64,
        timeout: u64,
    ) -> SCResult<()> {
        only_owner!(self, "Only owner may call this function!");
        let oracle_num = self.oracle_count(); // Save on storage reads
        require!(
            max_submissions >= min_submissions,
            "max must equal/exceed min"
        );
        require!(oracle_num >= max_submissions, "max cannot exceed total");
        require!(
            oracle_num == 0 || oracle_num > restart_delay,
            "delay cannot exceed total"
        );
        let recorded_funds = self.recorded_funds().value;
        require!(
            recorded_funds.available >= self.required_reserve(&payment_amount),
            "insufficient funds for payment"
        );
        require!(min_submissions > 0, "min must be greater than 0");

        set_value(self.payment_amount(), payment_amount);
        set_value(self.min_submission_count(), min_submissions);
        set_value(self.max_submission_count(), max_submissions);
        set_value(self.restart_delay(), restart_delay);
        set_value(self.timeout(), timeout);

        Ok(())
    }

    #[view]
    fn allocated_funds(&self) -> BigUint {
        self.recorded_funds().value.allocated
    }

    #[view]
    fn available_funds(&self) -> BigUint {
        self.recorded_funds().value.available
    }

    fn update_available_funds(&self) {
        let mut recorded_funds = self.recorded_funds();
        // TODO: use get balance for given token instead of get_sc_balance
        recorded_funds.value.available =
            self.get_sc_balance() - recorded_funds.value.allocated.clone();
        recorded_funds.save();
    }

    #[view]
    fn oracle_count(&self) -> u64 {
        self.oracle_addresses().len() as u64
    }

    #[view]
    fn get_round_data(&self, round_id: u64) -> SCResult<RoundData> {
        if let Some(r) = self.rounds().get(&round_id) {
            require!(r.answered_in_round > 0, "No data present");
            return Ok(RoundData {
                round_id,
                answer: r.answer,
                started_at: r.started_at,
                updated_at: r.updated_at,
                answered_in_round: r.answered_in_round,
            });
        }
        sc_error!("No data present")
    }

    #[view]
    fn latest_round_data(&self) -> SCResult<RoundData> {
        self.get_round_data(self.latest_round_id().value)
    }

    #[view]
    fn withdrawable_payment(&self, oracle: Address) -> SCResult<BigUint> {
        Ok(sc_try!(self.get_oracle_status(&oracle)).withdrawable)
    }

    fn withdraw_payment(
        &self,
        oracle: Address,
        recipient: Address,
        amount: BigUint,
    ) -> SCResult<()> {
        let mut oracle_status = sc_try!(self.get_oracle_status(&oracle));
        require!(
            oracle_status.admin == self.get_caller(),
            "only callable by admin"
        );

        require!(
            oracle_status.withdrawable >= amount,
            "insufficient withdrawable funds"
        );

        let mut recorded_funds = self.recorded_funds();
        oracle_status.withdrawable -= &amount;
        self.oracles().insert(oracle, oracle_status);
        recorded_funds.value.allocated -= &amount;
        recorded_funds.save();

        self.send()
            .direct(&recipient, &self.link_token().value, &amount, b"");
        Ok(())
    }

    fn withdraw_funds(&self, recipient: Address, amount: BigUint) -> SCResult<()> {
        only_owner!(self, "Only owner may call this function!");
        let recorded_funds = self.recorded_funds();
        require!(
            recorded_funds.value.available - self.required_reserve(&self.payment_amount().value)
                >= amount,
            "insufficient reserve funds"
        );
        self.send()
            .direct(&recipient, &self.link_token().value, &amount, b"");
        self.update_available_funds();
        Ok(())
    }

    #[view]
    fn get_admin(&self, oracle: Address) -> SCResult<Address> {
        Ok(sc_try!(self.get_oracle_status(&oracle)).admin)
    }

    #[endpoint]
    fn transfer_admin(&self, oracle: Address, new_admin: Address) -> SCResult<()> {
        let mut oracle_status = sc_try!(self.get_oracle_status(&oracle));
        require!(
            oracle_status.admin == self.get_caller(),
            "only callable by admin"
        );
        oracle_status.pending_admin = Some(new_admin);
        self.oracles().insert(oracle, oracle_status);
        Ok(())
    }

    #[endpoint]
    fn accept_admin(&self, oracle: Address) -> SCResult<()> {
        let mut oracle_status = sc_try!(self.get_oracle_status(&oracle));
        let caller = self.get_caller();
        require!(
            oracle_status.pending_admin == Some(caller.clone()),
            "only callable by pending admin"
        );
        oracle_status.pending_admin = None;
        oracle_status.admin = caller;
        self.oracles().insert(oracle, oracle_status);
        Ok(())
    }

    #[endpoint]
    fn request_new_round(&self) -> SCResult<u64> {
        let requester_option = self.requesters().get(&self.get_caller());
        require!(
            requester_option.map_or_else(|| false, |requester| requester.authorized),
            "not authorized requester"
        );

        let current = self.reporting_round_id().value;
        require!(
            self.rounds()
                .get(&current)
                .map_or_else(|| false, |round| round.updated_at > 0)
                || sc_try!(self.timed_out(&current)),
            "prev round must be supersedable"
        );

        let new_round_id = current + 1;
        sc_try!(self.requester_initialize_new_round(new_round_id));
        Ok(new_round_id)
    }

    fn set_requester_permissions(
        &self,
        requester: Address,
        authorized: bool,
        delay: u64,
    ) -> SCResult<()> {
        only_owner!(self, "Only owner may call this function!");
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
        Ok(())
    }

    #[view]
    fn oracle_round_state(
        &self,
        oracle: Address,
        queried_round_id: u64,
    ) -> SCResult<OracleRoundState<BigUint>> {
        if queried_round_id == 0 {
            return self.oracle_round_state_suggest_round(&oracle);
        }
        let eligible_to_submit =
            sc_try!(self.eligible_for_specific_round(&oracle, &queried_round_id));
        let round = sc_try!(self.get_round(&queried_round_id));
        let details = sc_try!(self.get_round_details(&queried_round_id));
        let oracle_status = sc_try!(self.get_oracle_status(&oracle));
        let recorded_funds = self.recorded_funds().value;
        Ok(OracleRoundState {
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
                self.payment_amount().value
            },
        })
    }

    fn initialize_new_round(&self, round_id: &u64) -> SCResult<()> {
        sc_try!(self.update_timed_out_round_info(round_id - 1));

        set_value(self.reporting_round_id(), round_id.clone());
        let next_details = RoundDetails {
            submissions: Vec::new(),
            max_submissions: self.max_submission_count().value,
            min_submissions: self.min_submission_count().value,
            timeout: self.timeout().value,
            payment_amount: self.payment_amount().value,
        };
        self.details().insert(round_id.clone(), next_details);
        self.rounds().insert(
            round_id.clone(),
            Round {
                answer: 0,
                started_at: self.get_block_timestamp(),
                updated_at: 0,
                answered_in_round: 0,
            },
        );
        Ok(())
    }

    fn oracle_initialize_new_round(&self, round_id: u64) -> SCResult<()> {
        sc_try!(self.new_round(&round_id));
        let oracle = self.get_caller();
        let mut oracle_status = sc_try!(self.get_oracle_status(&oracle));
        let restart_delay = self.restart_delay().value;
        if round_id <= oracle_status.last_started_round + restart_delay
            && oracle_status.last_started_round != 0
        {
            return Ok(());
        }

        sc_try!(self.initialize_new_round(&round_id));

        oracle_status.last_started_round = round_id;
        self.oracles().insert(oracle, oracle_status);
        Ok(())
    }

    fn requester_initialize_new_round(&self, round_id: u64) -> SCResult<()> {
        sc_try!(self.new_round(&round_id));
        let requester_address = self.get_caller();
        let mut requester = sc_try!(self.get_requester(&requester_address));
        require!(
            round_id > requester.last_started_round + requester.delay
                || requester.last_started_round == 0,
            "must delay requests"
        );

        sc_try!(self.initialize_new_round(&round_id));

        requester.last_started_round = round_id;
        self.requesters().insert(requester_address, requester);
        Ok(())
    }

    fn update_timed_out_round_info(&self, round_id: u64) -> SCResult<()> {
        sc_try!(self.new_round(&round_id));
        let mut round = sc_try!(self.get_round(&round_id));
        let prev_id = round_id - 1;
        let prev_round = sc_try!(self.get_round(&prev_id));
        round.answer = prev_round.answer;
        round.answered_in_round = prev_round.answered_in_round;
        round.updated_at = self.get_block_timestamp();
        self.rounds().insert(round_id, round);
        self.details().remove(&round_id);
        Ok(())
    }

    fn eligible_for_specific_round(
        &self,
        oracle: &Address,
        queried_round_id: &u64,
    ) -> SCResult<bool> {
        if self
            .rounds()
            .get(queried_round_id)
            .map_or_else(|| false, |round| round.started_at > 0)
        {
            Ok(sc_try!(self.accepting_submissions(&queried_round_id))
                && self.validate_oracle_round(oracle, queried_round_id).is_ok())
        } else {
            Ok(sc_try!(self.delayed(oracle, queried_round_id))
                && self.validate_oracle_round(oracle, queried_round_id).is_ok())
        }
    }

    fn oracle_round_state_suggest_round(
        &self,
        oracle: &Address,
    ) -> SCResult<OracleRoundState<BigUint>> {
        let oracle_status = sc_try!(self.get_oracle_status(oracle));

        let reporting_round_id = self.reporting_round_id().value;
        let should_supersede = oracle_status.last_reported_round == reporting_round_id
            || !sc_try!(self.accepting_submissions(&reporting_round_id));
        // Instead of nudging oracles to submit to the next round, the inclusion of
        // the shouldSupersede bool in the if condition pushes them towards
        // submitting in a currently open round.
        let mut eligible_to_submit: bool;
        let round: Round;
        let round_id: u64;
        let payment_amount: BigUint;
        if sc_try!(self.supersedable(&reporting_round_id)) && should_supersede {
            round_id = reporting_round_id + 1;
            round = sc_try!(self.get_round(&round_id));

            payment_amount = self.payment_amount().value;
            eligible_to_submit = sc_try!(self.delayed(&oracle, &round_id));
        } else {
            round_id = reporting_round_id;
            round = sc_try!(self.get_round(&round_id));

            let round_details = sc_try!(self.get_round_details(&round_id));
            payment_amount = round_details.payment_amount;
            eligible_to_submit = sc_try!(self.accepting_submissions(&round_id));
        }

        if self.validate_oracle_round(&oracle, &round_id).is_err() {
            eligible_to_submit = false;
        }

        let recorded_funds = self.recorded_funds().value;
        let round_details = sc_try!(self.get_round_details(&round_id));

        Ok(OracleRoundState {
            eligible_to_submit,
            round_id,
            latest_submission: oracle_status.latest_submission,
            started_at: round.started_at,
            timeout: round_details.timeout,
            available_funds: recorded_funds.available,
            oracle_count: self.oracle_count(),
            payment_amount,
        })
    }

    fn update_round_answer(&self, round_id: u64) -> SCResult<Option<u64>> {
        let details = sc_try!(self.get_round_details(&round_id));
        if (details.submissions.len() as u64) < details.min_submissions {
            return Ok(None);
        }

        let new_answer = median(details.submissions);
        let mut round = sc_try!(self.get_round(&round_id));
        round.answer = new_answer;
        round.updated_at = self.get_block_timestamp();
        round.answered_in_round = round_id;
        self.rounds().insert(round_id, round);
        set_value(self.latest_round_id(), round_id);

        return Ok(Some(new_answer));
    }

    fn pay_oracle(&self, round_id: u64) -> SCResult<()> {
        let round_details = sc_try!(self.get_round_details(&round_id));
        let oracle = self.get_caller();
        let mut oracle_status = sc_try!(self.get_oracle_status(&oracle));

        let payment = round_details.payment_amount;
        let mut recorded_funds = self.recorded_funds();
        recorded_funds.value.available -= &payment;
        recorded_funds.value.allocated += &payment;
        recorded_funds.save();

        oracle_status.withdrawable += &payment;
        self.oracles().insert(oracle, oracle_status);
        Ok(())
    }

    fn record_submission(&self, submission: u64, round_id: u64) -> SCResult<()> {
        require!(
            sc_try!(self.accepting_submissions(&round_id)),
            "round not accepting submissions"
        );

        let mut round_details = sc_try!(self.get_round_details(&round_id));
        let oracle = self.get_caller();
        let mut oracle_status = sc_try!(self.get_oracle_status(&oracle));
        round_details.submissions.push(submission);
        oracle_status.last_reported_round = round_id;
        oracle_status.latest_submission = submission;
        self.details().insert(round_id, round_details);
        self.oracles().insert(oracle, oracle_status);
        Ok(())
    }

    fn delete_round_details(&self, round_id: u64) {
        if let Some(details) = self.details().get(&round_id) {
            if (details.submissions.len() as u64) < details.max_submissions {
                return;
            }
        }

        self.details().remove(&round_id);
    }

    fn timed_out(&self, round_id: &u64) -> SCResult<bool> {
        let round = sc_try!(self.get_round(round_id));
        let started_at = round.started_at;
        let details = sc_try!(self.get_round_details(round_id));
        let round_timeout = details.timeout;
        Ok(started_at > 0
            && round_timeout > 0
            && started_at + round_timeout < self.get_block_timestamp())
    }

    fn get_starting_round(&self, oracle: &Address) -> SCResult<u64> {
        let current_round = self.reporting_round_id().value;
        let oracle_status = sc_try!(self.get_oracle_status(&oracle));
        if current_round != 0 && current_round == oracle_status.ending_round {
            return Ok(current_round);
        }
        Ok(current_round + 1)
    }

    fn previous_and_current_unanswered(&self, round_id: u64, rr_id: u64) -> SCResult<bool> {
        let round = sc_try!(self.get_round(&rr_id));
        Ok(round_id + 1 == rr_id && round.updated_at == 0)
    }

    #[view]
    fn required_reserve(&self, payment: &BigUint) -> BigUint {
        payment * &BigUint::from(self.oracle_count()) * BigUint::from(RESERVE_ROUNDS)
    }

    fn add_oracle(&self, oracle: &Address, admin: &Address) -> SCResult<()> {
        require!(!self.oracle_enabled(oracle), "oracle already enabled");

        self.oracles().insert(
            oracle.clone(),
            OracleStatus {
                withdrawable: BigUint::zero(),
                starting_round: sc_try!(self.get_starting_round(oracle)),
                ending_round: ROUND_MAX,
                last_reported_round: 0,
                last_started_round: 0,
                latest_submission: 0,
                admin: admin.clone(),
                pending_admin: None,
            },
        );
        Ok(())
    }

    fn validate_oracle_round(&self, oracle: &Address, round_id: &u64) -> SCResult<()> {
        let oracle_status = sc_try!(self.get_oracle_status(&oracle));
        let rr_id = self.reporting_round_id().value;

        require!(oracle_status.starting_round != 0, "not enabled oracle");
        require!(
            oracle_status.starting_round <= *round_id,
            "not yet enabled oracle"
        );
        require!(
            oracle_status.ending_round >= *round_id,
            "no longer allowed oracle"
        );
        require!(
            oracle_status.last_reported_round < *round_id,
            "cannot report on previous rounds"
        );
        require!(
            *round_id == rr_id
                || *round_id == rr_id + 1
                || sc_try!(self.previous_and_current_unanswered(*round_id, rr_id)),
            "invalid round to report"
        );
        require!(
            *round_id == 1 || sc_try!(self.supersedable(&(*round_id - 1))),
            "previous round not supersedable"
        );
        Ok(())
    }

    fn supersedable(&self, round_id: &u64) -> SCResult<bool> {
        let round = sc_try!(self.get_round(round_id));
        let timed_out = sc_try!(self.timed_out(round_id));
        Ok(round.updated_at > 0 || timed_out)
    }

    fn oracle_enabled(&self, oracle: &Address) -> bool {
        self.oracle_addresses().contains(oracle)
    }

    fn accepting_submissions(&self, round_id: &u64) -> SCResult<bool> {
        let details = sc_try!(self.get_round_details(round_id));
        Ok(details.max_submissions != 0)
    }

    fn delayed(&self, oracle: &Address, round_id: &u64) -> SCResult<bool> {
        let oracle_status = sc_try!(self.get_oracle_status(oracle));
        let last_started = oracle_status.last_started_round;
        Ok(*round_id > last_started + self.restart_delay().value || last_started == 0)
    }

    fn new_round(&self, round_id: &u64) -> SCResult<()> {
        require!(
            *round_id == self.reporting_round_id().value + 1,
            "the last round and the new round must be consecutive"
        );
        Ok(())
    }

    fn get_oracle_status(&self, oracle: &Address) -> SCResult<OracleStatus<BigUint>> {
        if let Some(oracle_status) = self.oracles().get(oracle) {
            return Ok(oracle_status);
        }
        sc_error!("No oracle at given address")
    }

    fn get_round(&self, round_id: &u64) -> SCResult<Round> {
        if let Some(round) = self.rounds().get(round_id) {
            return Ok(round);
        }
        sc_error!("No round for given round id")
    }

    fn get_round_details(&self, round_id: &u64) -> SCResult<RoundDetails<BigUint>> {
        if let Some(round_details) = self.details().get(round_id) {
            return Ok(round_details);
        }
        sc_error!("No round details for given round id")
    }

    fn get_requester(&self, requester_address: &Address) -> SCResult<Requester> {
        if let Some(requester) = self.requesters().get(requester_address) {
            return Ok(requester);
        }
        sc_error!("No requester has the given address")
    }
}
