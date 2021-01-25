use elrond_wasm::{Address, BoxedBytes, Vec};

derive_imports!();

#[derive(NestedEncode, NestedDecode, TopEncode, TopDecode, TypeAbi)]
pub struct OracleRequest {
    pub nonce_key: u64,
    pub caller_account: Address,
    pub callback_address: Address,
    pub callback_method: BoxedBytes,
    pub data: BoxedBytes,
}

#[derive(NestedEncode, NestedDecode, TopEncode, TopDecode, TypeAbi)]
pub struct AccountNonceData {
    pub address_key: Address,
    pub nonce_value: u64,
}

#[derive(NestedEncode, NestedDecode, TopEncode, TopDecode, TypeAbi)]
pub struct AccountRequestsData {
    pub address_key: Address,
    pub requests_value: Vec<OracleRequest>,
}
