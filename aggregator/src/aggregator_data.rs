multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub use crate::aggregator_interface::Submission;

pub const MAX_SUBMISSIONS: usize = 10;
pub type SubmissionsVec<M> = ArrayVec<Submission<M>, MAX_SUBMISSIONS>;

#[derive(NestedEncode, NestedDecode, TopEncode, TopDecode, TypeAbi)]
pub struct RoundDetails<M: ManagedTypeApi> {
    pub submissions: SubmissionsVec<M>,
    pub max_submissions: u64,
    pub min_submissions: u64,
    pub timeout: u64,
    pub payment_amount: BigUint<M>,
}

#[derive(NestedEncode, NestedDecode, TopEncode, TopDecode, TypeAbi)]
pub struct OracleStatus<M: ManagedTypeApi> {
    pub withdrawable: BigUint<M>,
    pub starting_round: u64,
    pub ending_round: u64,
    pub last_reported_round: u64,
    pub last_started_round: u64,
    pub latest_submission: Option<Submission<M>>,
    pub admin: ManagedAddress<M>,
    pub pending_admin: Option<ManagedAddress<M>>,
}

#[derive(NestedEncode, NestedDecode, TopEncode, TopDecode, TypeAbi)]
pub struct Requester {
    pub authorized: bool,
    pub delay: u64,
    pub last_started_round: u64,
}

#[derive(TopEncode, TopDecode, PartialEq, Clone)]
pub struct Funds<M: ManagedTypeApi> {
    pub available: BigUint<M>,
    pub allocated: BigUint<M>,
}

#[derive(NestedEncode, NestedDecode, TopEncode, TopDecode, TypeAbi)]
pub struct OracleRoundState<M: ManagedTypeApi> {
    pub eligible_to_submit: bool,
    pub round_id: u64,
    pub latest_submission: Option<Submission<M>>,
    pub started_at: u64,
    pub timeout: u64,
    pub available_funds: BigUint<M>,
    pub oracle_count: u64,
    pub payment_amount: BigUint<M>,
}

#[derive(ManagedVecItem)]
pub struct AddressAmountPair<M: ManagedTypeApi> {
    pub address: ManagedAddress<M>,
    pub amount: BigUint<M>,
}
