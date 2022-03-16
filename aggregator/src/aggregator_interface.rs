#![allow(non_snake_case)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

pub const MAX_SUBMISSION_VALUES: usize = 100;
pub type SingleSubmissionValuesVec<M> = ArrayVec<BigUint<M>, MAX_SUBMISSION_VALUES>;

pub const MAX_DESCRIPTION_LEN: usize = 50;
pub type DescriptionVec = ArrayVec<u8, MAX_DESCRIPTION_LEN>;

#[derive(NestedEncode, NestedDecode, TopEncode, TopDecode, TypeAbi, PartialEq, Debug, Clone)]
pub struct Submission<M: ManagedTypeApi> {
    pub values: SingleSubmissionValuesVec<M>,
}

#[derive(NestedEncode, NestedDecode, TopEncode, TopDecode, TypeAbi)]
pub struct Round<M: ManagedTypeApi> {
    pub round_id: u64,
    pub answer: Option<Submission<M>>,
    pub decimals: u8,
    pub description: DescriptionVec,
    pub started_at: u64,
    pub updated_at: u64,
    pub answered_in_round: u64,
}
