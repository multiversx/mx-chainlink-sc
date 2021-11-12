use elrond_wasm::{api::ManagedTypeApi, types::{ManagedAddress, BoxedBytes}};

elrond_wasm::derive_imports!();

#[derive(NestedEncode, NestedDecode, TopEncode, TopDecode, TypeAbi)]
pub struct OracleRequest<M: ManagedTypeApi> {
    pub caller: ManagedAddress<M>,
    pub callback_address: ManagedAddress<M>,
    pub callback_method: BoxedBytes,
    pub data: BoxedBytes,
}

#[derive(NestedEncode, NestedDecode, TopEncode, TopDecode, TypeAbi)]
pub struct RequestView<M: ManagedTypeApi> {
    pub address: ManagedAddress<M>,
    pub nonce: u64,
    pub data: BoxedBytes,
}
