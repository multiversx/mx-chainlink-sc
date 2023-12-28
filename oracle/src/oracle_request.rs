use multiversx_sc::{
    api::ManagedTypeApi,
    types::{ManagedAddress, ManagedBuffer},
};

multiversx_sc::derive_imports!();

#[derive(NestedEncode, NestedDecode, TopEncode, TopDecode, TypeAbi)]
pub struct OracleRequest<M: ManagedTypeApi> {
    pub caller: ManagedAddress<M>,
    pub callback_address: ManagedAddress<M>,
    pub callback_method: ManagedBuffer<M>,
    pub data: ManagedBuffer<M>,
}

#[derive(NestedEncode, NestedDecode, TopEncode, TopDecode, TypeAbi)]
pub struct RequestView<M: ManagedTypeApi> {
    pub address: ManagedAddress<M>,
    pub nonce: u64,
    pub data: ManagedBuffer<M>,
}
