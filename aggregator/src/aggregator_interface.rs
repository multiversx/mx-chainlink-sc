elrond_wasm::imports!();
elrond_wasm::derive_imports!();
use elrond_wasm::String;

#[derive(NestedEncode, NestedDecode, TopEncode, TopDecode, TypeAbi)]
pub struct Round<BigUint: BigUintApi> {
    pub round_id: u64,
    pub answer: BigUint,
    pub decimals: u8,
    pub description: String,
    pub started_at: u64,
    pub updated_at: u64,
    pub answered_in_round: u64,
}

#[elrond_wasm_derive::callable(AggregatorInterfaceProxy)]
pub trait AggregatorInterface<BigUint: BigUintApi> {
    fn submit(&self, round_id: u64, submission: BigUint) -> ContractCall<BigUint>;
    fn get_round_data(&self, round_id: u64) -> ContractCall<BigUint>; // Round
    fn latest_round_data(&self) -> ContractCall<BigUint>; // Round
}
