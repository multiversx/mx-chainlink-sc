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
    fn reply(&self, nonce: u64, answer: BoxedBytes) -> ContractCall<BigUint, ()>;
}

#[elrond_wasm_derive::contract(OracleImpl)]
pub trait Oracle {
    #[storage_mapper("nonces")]
    fn nonces(&self) -> MapMapper<Self::Storage, Address, u64>;

    #[storage_mapper("requests")]
    fn requests(
        &self,
    ) -> MapStorageMapper<Self::Storage, Address, MapMapper<Self::Storage, u64, OracleRequest>>;

    #[view(requestsAsVec)]
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

    #[view(authorizedNodes)]
    #[storage_mapper("authorized_nodes")]
    fn authorized_nodes(&self) -> SetMapper<Self::Storage, Address>;

    #[init]
    fn init(&self) {}

    /// This is the entry point that will use the escrow transfer_from.
    /// Afterwards, it essentially calls itself (store_request) which stores the request in state.
    #[endpoint(request)]
    fn request(
        &self,
        callback_address: Address,
        callback_method: BoxedBytes,
        nonce: u64,
        data: BoxedBytes,
    ) -> SCResult<()> {
        let caller = self.blockchain().get_caller();
        let mut requests = self.requests();
        let mut caller_requests = match requests.get(&caller) {
            Some(req) => req,
            None => {
                requests.insert_default(caller.clone());

                requests.get(&caller).unwrap()
            }
        };

        // Ensure there isn't already the same nonce
        if caller_requests.contains_key(&nonce) {
            return sc_error!("Existing account and nonce in requests");
        }

        let mut nonces = self.nonces();
        let expected_nonce = nonces.get(&caller).map_or(0, |last_nonce| last_nonce + 1);
        require!(nonce == expected_nonce, "Invalid nonce");

        // store request
        let new_request = OracleRequest {
            caller: caller.clone(),
            callback_address,
            callback_method,
            data,
        };
        caller_requests.insert(nonce, new_request);
        nonces.insert(caller, nonce);

        Ok(())
    }

    #[endpoint(fulfillRequest)]
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

    #[endpoint(submit)]
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

    #[endpoint(addAuthorization)]
    fn add_authorization(&self, node: Address) -> SCResult<()> {
        only_owner!(self, "Caller must be owner");
        require!(self.authorized_nodes().insert(node), "Already authorized");
        Ok(())
    }

    #[endpoint(removeAuthorization)]
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
            self.authorized_nodes().contains(&self.blockchain().get_caller()),
            "Not an authorized node to fulfill requests."
        );
        Ok(())
    }
}
