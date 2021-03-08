#![no_std]

extern crate aggregator;
use crate::aggregator::aggregator_interface::{AggregatorInterface, AggregatorInterfaceProxy};
use elrond_wasm::types::MultiResultVec;
mod oracle_request;
use oracle_request::{OracleRequest, RequestView};

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[elrond_wasm_derive::callable(ClientInterfaceProxy)]
pub trait ClientInterface<BigUint: BigIntApi> {
    fn reply(&self, nonce: u64, answer: BoxedBytes) -> ContractCall<BigUint>;
}

#[elrond_wasm_derive::contract(OracleImpl)]
pub trait Oracle {
    #[storage_mapper("nonces")]
    fn nonces(&self) -> MapMapper<Self::Storage, Address, u64>;

    #[storage_mapper("requests")]
    fn requests(
        &self,
    ) -> MapStorageMapper<Self::Storage, Address, MapMapper<Self::Storage, u64, OracleRequest>>;

    #[view]
    fn requests_as_vec(&self) -> MultiResultVec<RequestView> {
        let mut vec: Vec<RequestView> = Vec::new();
        for (address, request) in self.requests().iter() {
            for (nonce, oracle_request) in request.iter() {
                vec.push(RequestView {
                    address: address.clone(),
                    nonce,
                    data: oracle_request.data,
                })
            }
        }
        vec.into()
    }

    #[view]
    #[storage_mapper("authorized_nodes")]
    fn authorized_nodes(&self) -> SetMapper<Self::Storage, Address>;

    #[init]
    fn init(&self) -> SCResult<()> {
        Ok(())
    }

    /// This is the entry point that will use the escrow transfer_from.
    /// Afterwards, it essentially calls itself (store_request) which stores the request in state.
    #[endpoint]
    fn request(
        &self,
        callback_address: Address,
        callback_method: BoxedBytes,
        nonce: u64,
        data: BoxedBytes,
    ) -> SCResult<()> {
        let caller = self.get_caller();
        let mut requests = self.requests();
        let mut caller_requests = requests.get_or_insert_default(caller.clone());

        // Ensure there isn't already the same nonce
        if caller_requests.contains_key(&nonce) {
            return sc_error!("Existing account and nonce in requests");
        }

        let mut nonces = self.nonces();
        if let Some(last_nonce) = nonces.get(&caller) {
            require!(last_nonce < nonce, "Invalid, already used nonce");
        }

        // store request
        let new_request = OracleRequest {
            caller_account: caller.clone(),
            callback_address,
            callback_method,
            data,
        };
        caller_requests.insert(nonce, new_request);
        nonces.insert(caller, nonce);

        Ok(())
    }

    /// Note that the request_id here is String instead of Vec<u8> as might be expected from the Solidity contract
    #[endpoint]
    fn fulfill_request(
        &self,
        address: Address,
        nonce: u64,
        data: BoxedBytes,
    ) -> SCResult<AsyncCall<BigUint>> {
        sc_try!(self.only_authorized_node());

        // Get the request
        let requests = self.requests();
        let address_requests_option = requests.get(&address);

        require!(
            address_requests_option.is_some(),
            "Did not find the account to fulfill."
        );
        let mut address_requests = address_requests_option.unwrap();

        let request_option = address_requests.get(&nonce);
        require!(
            request_option.is_some(),
            "Did not find the request (nonce) to fulfill."
        );
        let request = request_option.unwrap();

        address_requests.remove(&nonce);

        let client = contract_call!(self, request.callback_address, ClientInterfaceProxy);
        Ok(client.reply(nonce, data).async_call())
    }

    #[endpoint]
    fn submit(
        &self,
        aggregator: Address,
        round_id: u64,
        submission: BigUint,
    ) -> SCResult<AsyncCall<BigUint>> {
        only_owner!(self, "Only owner may call this function!");
        Ok(contract_call!(self, aggregator, AggregatorInterfaceProxy)
            .submit(round_id, submission)
            .async_call())
    }

    #[endpoint]
    fn add_authorization(&self, node: Address) -> SCResult<()> {
        only_owner!(self, "Caller must be owner");
        self.authorized_nodes().insert(node);
        Ok(())
    }

    #[endpoint]
    fn remove_authorization(&self, node: Address) -> SCResult<()> {
        only_owner!(self, "Caller must be owner");
        require!(
            self.authorized_nodes().remove(&node),
            "Authorization not found"
        );
        Ok(())
    }

    fn only_authorized_node(&self) -> SCResult<()> {
        require!(
            self.authorized_nodes().contains(&self.get_caller()),
            "Not an authorized node to fulfill requests."
        );
        Ok(())
    }
}
