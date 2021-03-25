elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[derive(NestedEncode, NestedDecode, TopEncode, TopDecode, TypeAbi, PartialEq, Debug, Clone)]
pub struct Submission<BigUint: BigUintApi> {
    pub values: Vec<BigUint>,
}

#[derive(NestedEncode, NestedDecode, TopEncode, TopDecode, TypeAbi)]
pub struct Round<BigUint: BigUintApi> {
    pub round_id: u64,
    pub answer: Option<Submission<BigUint>>,
    pub decimals: u8,
    pub description: BoxedBytes,
    pub started_at: u64,
    pub updated_at: u64,
    pub answered_in_round: u64,
}

#[elrond_wasm_derive::callable(AggregatorInterfaceProxy)]
pub trait AggregatorInterface<BigUint: BigUintApi> {
    fn submit(&self, round_id: u64, submission: BigUint) -> ContractCall<BigUint, ()>;
    fn get_round_data(&self, round_id: u64) -> ContractCall<BigUint, ()>; // OptionalArg<Round<BigUint>>
    fn latest_round_data(&self) -> ContractCall<BigUint, ()>; // OptionalArg<Round<BigUint>>
}
