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

// Optional: Use SingleValueMapper instead
#[elrond_wasm_derive::contract(ClientImpl)]
pub trait Client<BigUint: BigUintApi> {
    #[storage_get("oracle_address")]
    fn get_oracle_address(&self) -> Address;

    #[storage_set("oracle_address")]
    fn set_oracle_address(&self, oracle_address: Address);

    // Don't use Option, instead store only ClientData, and add an is_empty storage checker,
    // (automatically added if you use SingleValueMapper)
    // Then make a separate view function that returns it as Option<>
    #[view]
    #[storage_get("client_data")]
    fn get_client_data(&self) -> Option<ClientData>;

    #[storage_set("client_data")]
    fn set_client_data(&self, user_data: Option<ClientData>);

    // There is no need to return a SCResult here
    #[init]
    fn init(&self, oracle_address: Address) -> SCResult<()> {
        self.set_oracle_address(oracle_address);
        Ok(())
    }

    #[endpoint]
    fn send_request(&self) -> SCResult<AsyncCall<BigUint>> {
        only_owner!(self, "Caller must be owner");
        let callback_address = self.get_sc_address();
        // There is no need to concat. You can simply use
        // BoxedBytes::from(&b"reply"[..])
        let callback_method = BoxedBytes::from_concat(&[b"reply"]);
        let nonce = self.get_block_nonce();
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
        self.set_client_data(Some(ClientData { nonce, answer }));
        Ok(())
    }
}
