elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[derive(NestedEncode, NestedDecode, TopEncode, TopDecode, TypeAbi, Clone)]
pub struct TokenPair<M: ManagedTypeApi> {
    pub from: ManagedBuffer<M>,
    pub to: ManagedBuffer<M>,
}

#[derive(NestedEncode, NestedDecode, TopEncode, TopDecode, TypeAbi)]
pub struct PriceFeed<M: ManagedTypeApi> {
    pub round_id: u32,
    pub from: ManagedBuffer<M>,
    pub to: ManagedBuffer<M>,
    pub price: BigUint<M>,
    pub decimals: u8,
}

#[derive(NestedEncode, NestedDecode, TopEncode, TopDecode, TypeAbi)]
pub struct OracleStatus {
    pub accepted_submissions: u64,
    pub total_submissions: u64,
}
