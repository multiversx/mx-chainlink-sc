elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[derive(NestedEncode, NestedDecode, TopEncode, TopDecode, TypeAbi)]
pub struct ClientData {
    pub nonce: u64,
    pub answer: BoxedBytes,
}
