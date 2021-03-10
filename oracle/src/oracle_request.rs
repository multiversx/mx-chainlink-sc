use elrond_wasm::types::{Address, BoxedBytes};

elrond_wasm::derive_imports!();

#[derive(NestedEncode, NestedDecode, TopEncode, TopDecode, TypeAbi)]
pub struct OracleRequest {
    pub caller_account: Address, // Optional: Rename to "caller" instead of "caller_account"
    pub callback_address: Address,
    pub callback_method: BoxedBytes,
    pub data: BoxedBytes,
}

#[derive(NestedEncode, NestedDecode, TopEncode, TopDecode, TypeAbi)]
pub struct RequestView {
    pub address: Address,
    pub nonce: u64,
    pub data: BoxedBytes,
}
