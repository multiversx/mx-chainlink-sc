#![no_std]

mod client_data;
use client_data::ClientData;

elrond_wasm::imports!();

#[elrond_wasm_derive::callable(OracleInterfaceProxy)]
pub trait OracleInterface {
    fn request(
        &self,
        callback_address: Address,
        callback_method: BoxedBytes,
        nonce: u64,
        data: BoxedBytes,
    ) -> ContractCall<BigUint>;
}

#[elrond_wasm_derive::contract(ClientImpl)]
pub trait Client<BigUint: BigUintApi> {
    #[storage_get("oracle_address")]
    fn get_oracle_address(&self) -> Address;

    #[storage_set("oracle_address")]
    fn set_oracle_address(&self, oracle_address: Address);

    #[view]
    fn get_client_data(&self) -> Option<ClientData> {
        if self.client_data().is_empty() {
            None
        } else {
            Some(self.client_data().get())
        }
    }

    #[storage_mapper("client_data")]
    fn client_data(&self) -> SingleValueMapper<Self::Storage, ClientData>;

    #[storage_mapper("nonce")]
    fn nonce(&self) -> SingleValueMapper<Self::Storage, u64>;

    #[storage_set("client_data")]
    fn set_client_data(&self, user_data: ClientData);

    #[init]
    fn init(&self, oracle_address: Address) {
        self.set_oracle_address(oracle_address);
    }

    #[endpoint]
    fn send_request(&self) -> SCResult<AsyncCall<BigUint>> {
        only_owner!(self, "Caller must be owner");
        let callback_address = self.get_sc_address();
        let callback_method = BoxedBytes::from(&b"reply"[..]);
        let nonce = self.nonce().get();
        self.nonce().update(|nonce| *nonce += 1);
        let data = BoxedBytes::empty();
        let oracle = contract_call!(self, self.get_oracle_address(), OracleInterfaceProxy);
        Ok(oracle
            .request(callback_address, callback_method, nonce, data)
            .async_call())
    }

    #[endpoint]
    fn reply(&self, nonce: u64, answer: BoxedBytes) -> SCResult<()> {
        require!(
            self.get_caller() == self.get_oracle_address(),
            "Only oracle can reply"
        );
        self.client_data().set(&ClientData { nonce, answer });
        Ok(())
    }
}
