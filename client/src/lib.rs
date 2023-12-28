#![no_std]

mod client_data;
use client_data::ClientData;

multiversx_sc::imports!();

#[multiversx_sc::contract]
pub trait Client {
    #[storage_mapper("oracle_address")]
    fn oracle_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getClientData)]
    fn get_client_data(&self) -> OptionalValue<ClientData<Self::Api>> {
        if self.client_data().is_empty() {
            OptionalValue::None
        } else {
            OptionalValue::Some(self.client_data().get())
        }
    }

    #[storage_mapper("client_data")]
    fn client_data(&self) -> SingleValueMapper<ClientData<Self::Api>>;

    #[storage_mapper("nonce")]
    fn nonce(&self) -> SingleValueMapper<u64>;

    #[proxy]
    fn oracle_proxy(&self, to: ManagedAddress) -> oracle::Proxy<Self::Api>;

    #[init]
    fn init(&self, oracle_address: ManagedAddress) {
        self.oracle_address().set(oracle_address);
    }

    #[only_owner]
    #[endpoint(sendRequest)]
    fn send_request(&self) {
        let callback_address = self.blockchain().get_sc_address();
        let callback_method = ManagedBuffer::from(&b"reply"[..]);
        let nonce = self.nonce().get();
        self.nonce().update(|nonce| *nonce += 1);
        let data = ManagedBuffer::new();
        let mut oracle = self.oracle_proxy(self.oracle_address().get());

        oracle
            .request(callback_address, callback_method, nonce, data)
            .async_call()
            .call_and_exit();
    }

    #[endpoint(reply)]
    fn reply(&self, nonce: u64, answer: ManagedBuffer) {
        require!(
            self.blockchain().get_caller() == self.oracle_address().get(),
            "Only oracle can reply"
        );
        self.client_data().set(&ClientData { nonce, answer });
    }
}
