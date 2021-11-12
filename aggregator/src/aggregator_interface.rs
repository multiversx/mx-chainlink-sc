#![allow(non_snake_case)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[derive(NestedEncode, NestedDecode, TopEncode, TopDecode, TypeAbi, PartialEq, Debug, Clone)]
pub struct Submission<M: ManagedTypeApi> {
    pub values: Vec<BigUint<M>>,
}

#[derive(NestedEncode, NestedDecode, TopEncode, TopDecode, TypeAbi)]
pub struct Round<M: ManagedTypeApi> {
    pub round_id: u64,
    pub answer: Option<Submission<M>>,
    pub decimals: u8,
    pub description: BoxedBytes,
    pub started_at: u64,
    pub updated_at: u64,
    pub answered_in_round: u64,
}
