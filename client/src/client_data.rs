elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[derive(NestedEncode, NestedDecode, TopEncode, TopDecode, TypeAbi)]
pub struct ClientData<M: ManagedTypeApi> {
    pub nonce: u64,
    pub answer: ManagedBuffer<M>,
}
