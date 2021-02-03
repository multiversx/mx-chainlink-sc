use elrond_wasm::{Address, BoxedBytes};

derive_imports!();

#[derive(NestedEncode, NestedDecode, TopEncode, TopDecode, TypeAbi)]
pub struct OracleRequest {
    pub caller_account: Address,
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
