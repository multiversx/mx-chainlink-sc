elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[derive(NestedEncode, NestedDecode, TopEncode, TopDecode, TypeAbi, Clone)]
pub struct TokenPair {
    pub from: BoxedBytes,
    pub to: BoxedBytes,
}

#[derive(NestedEncode, NestedDecode, TopEncode, TopDecode, TypeAbi)]
pub struct PriceFeed<BigUint: BigUintApi> {
    pub round_id: u32,
    pub from: BoxedBytes,
    pub to: BoxedBytes,
    pub price: BigUint,
    pub decimals: u8,
}

#[derive(NestedEncode, NestedDecode, TopEncode, TopDecode, TypeAbi)]
pub struct OracleStatus {
    pub accepted_submissions: u64,
    pub total_submissions: u64,
}
