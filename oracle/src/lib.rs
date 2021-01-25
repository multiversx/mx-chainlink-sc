//#![no_std]

mod oracle_data;
use elrond_wasm::String;
use oracle_data::{AccountNonceData, AccountRequestsData, OracleRequest};

imports!();

#[elrond_wasm_derive::callable(ClientInterface)]
pub trait ClientInterface {
    fn reply(&self, nonce: u64, answer: BoxedBytes) -> SCResult<()>;
}

#[elrond_wasm_derive::contract(OracleImpl)]
pub trait Oracle {
    #[view]
    #[storage_get("nonces")]
    fn get_nonces(&self) -> Vec<AccountNonceData>;

    #[storage_set("nonces")]
    fn set_nonces(&self, nonces: Vec<AccountNonceData>);

    #[view]
    #[storage_get("requests")]
    fn get_requests(&self) -> Vec<AccountRequestsData>;

    #[storage_set("requests")]
    fn set_requests(&self, requests: Vec<AccountRequestsData>);

    #[view]
    #[storage_get("authorized_nodes")]
    fn get_authorized_nodes(&self) -> Vec<Address>;

    #[storage_set("authorized_nodes")]
    fn set_authorized_nodes(&self, authorized_nodes: Vec<Address>);

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
        let mut requests = self.get_requests();
        if let Some(caller_requests) = self.find_requests_by_caller(&mut requests, &caller) {
            // Ensure there isn't already the same nonce
            if self.find_request_by_nonce(caller_requests, nonce).is_some() {
                return sc_error!("Existing account and nonce in requests");
            }
        }

        if let Some(last_nonce) = self.find_nonce_by_caller(&self.get_nonces(), &caller) {
            require!(last_nonce < nonce, "Invalid, already used nonce");
        }

        // store request
        let oracle_request = OracleRequest {
            nonce_key: nonce,
            caller_account: caller.clone(),
            callback_address,
            callback_method,
            data,
        };

        // Insert request and commitment into state.
        /*
            account =>
            nonce => { Request }
        */
        if let Some(caller_requests) = self.find_requests_by_caller(&mut requests, &caller) {
            caller_requests.push(oracle_request);
        } else {
            let mut nonce_request = AccountRequestsData {
                address_key: caller.clone(),
                requests_value: Vec::new(),
            };
            nonce_request.requests_value.push(oracle_request);
            requests.push(nonce_request);
        }
        let mut nonces = self.get_nonces();
        nonces.push(AccountNonceData {
            address_key: caller,
            nonce_value: nonce,
        });

        self.set_nonces(nonces);
        self.set_requests(requests);

        Ok(())
    }

    /// Note that the request_id here is String instead of Vec<u8> as might be expected from the Solidity contract
    #[endpoint]
    fn fulfill_request(&self, address: Address, nonce: u64, data: BoxedBytes) -> SCResult<()> {
        sc_try!(self.only_authorized_node());

        // Get the request
        let mut requests = self.get_requests();
        let address_requests_option = self.find_requests_by_caller(&mut requests, &address);

        require!(
            address_requests_option.is_some(),
            "Did not find the account to fulfill."
        );
        let address_requests = address_requests_option.unwrap();

        let request_option = self.find_request_by_nonce(address_requests, nonce);
        require!(
            request_option.is_some(),
            "Did not find the request (nonce) to fulfill."
        );
        let request = request_option.unwrap();

        let client = contract_proxy!(self, &request.callback_address, ClientInterface);
        client.reply(nonce, data);

        // Remove request from state
        self.remove_request_by_nonce(address_requests, nonce);
        self.set_requests(requests);
        Ok(())
    }

    fn find_nonce_by_caller(
        &self,
        nonces: &Vec<AccountNonceData>,
        address: &Address,
    ) -> Option<u64> {
        nonces
            .iter()
            .find(|&entry| entry.address_key == *address)
            .map(|entry| entry.nonce_value)
    }

    fn find_requests_by_caller<'a>(
        &self,
        requests: &'a mut Vec<AccountRequestsData>,
        address: &Address,
    ) -> Option<&'a mut Vec<OracleRequest>> {
        requests
            .iter_mut()
            .find(|entry| entry.address_key == *address)
            .map(|entry| &mut entry.requests_value)
    }

    fn find_request_by_nonce<'a>(
        &self,
        requests: &'a mut Vec<OracleRequest>,
        nonce: u64,
    ) -> Option<&'a mut OracleRequest> {
        requests.iter_mut().find(|entry| entry.nonce_key == nonce)
    }

    fn remove_request_by_nonce(&self, requests: &mut Vec<OracleRequest>, nonce: u64) {
        if let Some(pos) = requests.iter().position(|entry| entry.nonce_key == nonce) {
            requests.remove(pos);
        }
    }

    #[endpoint]
    fn add_authorization(&self, node: Address) -> SCResult<()> {
        only_owner!(self, "Caller must be owner");
        let mut authorized_nodes = self.get_authorized_nodes();
        authorized_nodes.push(node);
        self.set_authorized_nodes(authorized_nodes);
        Ok(())
    }

    #[endpoint]
    fn remove_authorization(&self, node: Address) -> SCResult<()> {
        only_owner!(self, "Caller must be owner");
        let mut authorized_nodes = self.get_authorized_nodes();
        if let Some(pos) = authorized_nodes.iter().position(|entry| *entry == node) {
            authorized_nodes.remove(pos);
            self.set_authorized_nodes(authorized_nodes);
            return Ok(());
        }
        sc_error!("Authorization not found")
    }

    fn only_authorized_node(&self) -> SCResult<()> {
        let caller = self.get_caller();
        let authorized_nodes = self.get_authorized_nodes();
        require!(
            authorized_nodes
                .iter()
                .find(|&entry| entry == &caller)
                .is_some(),
            "Not an authorized node to fulfill requests."
        );
        Ok(())
    }

    #[view]
    fn get_requests_json(&self) -> String {
        let requests = self.get_requests();
        let requests_json = requests
            .iter()
            .map(|account_data| {
                account_data.requests_value.iter().map(move |request| {
                    format!(
                        "{{\"address\":\"{:02x}\", \"nonce\":\"{}\", \"data\":\"{:02x}\"}}",
                        ByteBuf(account_data.address_key.as_ref()),
                        request.nonce_key,
                        ByteBuf(request.data.as_slice())
                    )
                })
            })
            .flatten()
            .collect::<Vec<String>>()
            .join(", ");
        format!("[{}]", requests_json)
    }
}
struct ByteBuf<'a>(&'a [u8]);

impl<'a> std::fmt::LowerHex for ByteBuf<'a> {
    fn fmt(&self, fmtr: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        for byte in self.0 {
            fmtr.write_fmt(format_args!("{:02x}", byte))?;
        }
        Result::Ok(())
    }
}
