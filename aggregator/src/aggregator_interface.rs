elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[elrond_wasm_derive::callable(AggregatorInterfaceProxy)]
pub trait AggregatorInterface<BigUint: BigUintApi> {
    fn get_info() -> ContractCall<BigUint>; // Info
    fn get_round_data(&self, round_id: u64) -> ContractCall<BigUint>; // Round
    fn latest_round_data(&self) -> ContractCall<BigUint>; // Round
}
