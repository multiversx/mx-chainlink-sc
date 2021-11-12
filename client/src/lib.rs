#![no_std]

mod client_data;
use client_data::ClientData;

elrond_wasm::imports!();

#[elrond_wasm::derive::contract]
pub trait Client {
    #[storage_get("oracle_address")]
    fn get_oracle_address(&self) -> ManagedAddress;

    #[storage_set("oracle_address")]
    fn set_oracle_address(&self, oracle_address: ManagedAddress);

    #[view(getClientData)]
    fn get_client_data(&self) -> OptionalResult<ClientData> {
        if self.client_data().is_empty() {
            OptionalResult::None
        } else {
            OptionalResult::Some(self.client_data().get())
        }
    }

    #[storage_mapper("client_data")]
    fn client_data(&self) -> SingleValueMapper<ClientData>;

    #[storage_mapper("nonce")]
    fn nonce(&self) -> SingleValueMapper<u64>;

    #[storage_set("client_data")]
    fn set_client_data(&self, user_data: ClientData);

    #[proxy]
    fn oracle_proxy(&self, to: ManagedAddress) -> oracle::Proxy<Self::Api>;

    #[init]
    fn init(&self, oracle_address: ManagedAddress) {
        self.set_oracle_address(oracle_address);
    }

    #[endpoint(sendRequest)]
    fn send_request(&self) -> SCResult<AsyncCall> {
        only_owner!(self, "Caller must be owner");
        let callback_address = self.blockchain().get_sc_address();
        let callback_method = BoxedBytes::from(&b"reply"[..]);
        let nonce = self.nonce().get();
        self.nonce().update(|nonce| *nonce += 1);
        let data = BoxedBytes::empty();
        let oracle = self.oracle_proxy(self.get_oracle_address());
        Ok(oracle
            .request(callback_address, callback_method, nonce, data)
            .async_call())
    }

    #[endpoint(reply)]
    fn reply(&self, nonce: u64, answer: BoxedBytes) -> SCResult<()> {
        require!(
            self.blockchain().get_caller() == self.get_oracle_address(),
            "Only oracle can reply"
        );
        self.client_data().set(&ClientData { nonce, answer });
        Ok(())
    }
}
