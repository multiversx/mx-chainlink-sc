#![no_std]

extern crate aggregator;
mod oracle_request;
use oracle_request::{OracleRequest, RequestView};

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

mod client_proxy {
    multiversx_sc::imports!();
    #[multiversx_sc::derive::proxy]
    pub trait Client {
        #[endpoint]
        fn reply(&self, nonce: u64, answer: ManagedBuffer);
    }
}

#[multiversx_sc::contract]
pub trait Oracle {
    #[storage_mapper("nonces")]
    fn nonces(&self) -> MapMapper<ManagedAddress, u64>;

    #[storage_mapper("requests")]
    fn requests(
        &self,
    ) -> MapStorageMapper<ManagedAddress, MapMapper<u64, OracleRequest<Self::Api>>>;

    #[view(requestsAsVec)]
    fn requests_as_vec(&self) -> MultiValueEncoded<RequestView<Self::Api>> {
        let mut vec = MultiValueEncoded::new();
        for (address, request) in self.requests().iter() {
            for (nonce, oracle_request) in request.iter() {
                vec.push(RequestView {
                    address: address.clone(),
                    nonce,
                    data: oracle_request.data,
                })
            }
        }

        vec
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
        callback_method: ManagedBuffer,
        nonce: u64,
        data: ManagedBuffer,
    ) {
        let caller = self.blockchain().get_caller();
        let mut requests = self.requests();
        let mut caller_requests = requests.entry(caller.clone()).or_default().get();

        // Ensure there isn't already the same nonce
        require!(
            !caller_requests.contains_key(&nonce),
            "Existing account and nonce in requests"
        );

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
    }

    #[endpoint(fulfillRequest)]
    fn fulfill_request(&self, address: ManagedAddress, nonce: u64, data: ManagedBuffer) {
        self.only_authorized_node();

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

        let mut client = self.client_proxy(request.callback_address);
        client.reply(nonce, data).async_call().call_and_exit();
    }

    #[only_owner]
    #[endpoint(submit)]
    fn submit(&self, aggregator: ManagedAddress, round_id: u64, submission: BigUint) {
        let mut submission_values = MultiValueEncoded::new();
        submission_values.push(submission);

        self.aggregator_proxy(aggregator)
            .submit(round_id, submission_values)
            .async_call()
            .call_and_exit();
    }

    #[only_owner]
    #[endpoint(addAuthorization)]
    fn add_authorization(&self, node: ManagedAddress) {
        require!(self.authorized_nodes().insert(node), "Already authorized");
    }

    #[only_owner]
    #[endpoint(removeAuthorization)]
    fn remove_authorization(&self, node: ManagedAddress) {
        require!(
            self.authorized_nodes().remove(&node),
            "Authorization not found"
        );
    }

    fn only_authorized_node(&self) {
        require!(
            self.authorized_nodes()
                .contains(&self.blockchain().get_caller()),
            "Not an authorized node to fulfill requests."
        );
    }

    #[proxy]
    fn client_proxy(&self, to: ManagedAddress) -> client_proxy::Proxy<Self::Api>;

    #[proxy]
    fn aggregator_proxy(&self, to: ManagedAddress) -> aggregator::Proxy<Self::Api>;
}
