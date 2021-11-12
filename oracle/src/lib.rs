#![no_std]

extern crate aggregator;
use elrond_wasm::types::MultiResultVec;
mod oracle_request;
use oracle_request::{OracleRequest, RequestView};

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

mod client_proxy {
    elrond_wasm::imports!();
    #[elrond_wasm::derive::proxy]
    pub trait Client {
        #[endpoint]
        fn reply(&self, nonce: u64, answer: BoxedBytes);
    }
}

#[elrond_wasm::derive::contract]
pub trait Oracle {
    #[storage_mapper("nonces")]
    fn nonces(&self) -> MapMapper<ManagedAddress, u64>;

    #[storage_mapper("requests")]
    fn requests(
        &self,
    ) -> MapStorageMapper<ManagedAddress, MapMapper<u64, OracleRequest<Self::Api>>>;

    #[view(requestsAsVec)]
    fn requests_as_vec(&self) -> MultiResultVec<RequestView<Self::Api>> {
        let mut vec: Vec<RequestView<Self::Api>> = Vec::new();
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
    fn authorized_nodes(&self) -> SetMapper<ManagedAddress>;

    #[init]
    fn init(&self) {}

    /// This is the entry point that will use the escrow transfer_from.
    /// Afterwards, it essentially calls itself (store_request) which stores the request in state.
    #[endpoint(request)]
    fn request(
        &self,
        callback_address: ManagedAddress,
        callback_method: BoxedBytes,
        nonce: u64,
        data: BoxedBytes,
    ) -> SCResult<()> {
        let caller = self.blockchain().get_caller();
        let mut requests = self.requests();
        let mut caller_requests = requests.entry(caller.clone()).or_default().get();

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
        address: ManagedAddress,
        nonce: u64,
        data: BoxedBytes,
    ) -> SCResult<AsyncCall> {
        self.only_authorized_node()?;

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

        let client = self.client_proxy(request.callback_address);
        Ok(client.reply(nonce, data).async_call())
    }

    #[endpoint(submit)]
    fn submit(
        &self,
        aggregator: ManagedAddress,
        round_id: u64,
        submission: BigUint,
    ) -> SCResult<AsyncCall> {
        only_owner!(self, "Only owner may call this function!");
        Ok(self
            .aggregator_proxy(aggregator)
            .submit(round_id, [submission].iter().cloned().collect())
            .async_call())
    }

    #[endpoint(addAuthorization)]
    fn add_authorization(&self, node: ManagedAddress) -> SCResult<()> {
        only_owner!(self, "Caller must be owner");
        require!(self.authorized_nodes().insert(node), "Already authorized");
        Ok(())
    }

    #[endpoint(removeAuthorization)]
    fn remove_authorization(&self, node: ManagedAddress) -> SCResult<()> {
        only_owner!(self, "Caller must be owner");
        require!(
            self.authorized_nodes().remove(&node),
            "Authorization not found"
        );
        Ok(())
    }

    fn only_authorized_node(&self) -> SCResult<()> {
        require!(
            self.authorized_nodes()
                .contains(&self.blockchain().get_caller()),
            "Not an authorized node to fulfill requests."
        );
        Ok(())
    }

    #[proxy]
    fn client_proxy(&self, to: ManagedAddress) -> client_proxy::Proxy<Self::Api>;

    #[proxy]
    fn aggregator_proxy(&self, to: ManagedAddress) -> aggregator::Proxy<Self::Api>;
}
