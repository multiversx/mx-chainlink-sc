#![no_std]

mod client_data;
use client_data::{ClientData};

imports!();

#[elrond_wasm_derive::callable(OracleInterface)]
pub trait OracleInterface {
    fn request(&self, callback_address: Address, callback_method: BoxedBytes, nonce: u64, data: BoxedBytes) -> SCResult<()>;
}

#[elrond_wasm_derive::contract(ClientImpl)]
pub trait Client {
	#[storage_get("oracle_address")]
	fn get_oracle_address(&self) -> Address;

	#[storage_set("oracle_address")]
	fn set_oracle_address(&self, oracle_address : Address);

	#[view]
	#[storage_get("client_data")]
	fn get_client_data(&self) -> Option<ClientData>;

	#[storage_set("client_data")]
	fn set_client_data(&self, user_data: Option<ClientData>);

	#[init]
	fn init(&self, oracle_address: Address) -> SCResult<()> {
		self.set_oracle_address(oracle_address);
		Ok(())
	}

	#[endpoint]
	fn send_request(&self) -> SCResult<()> {
		only_owner!(self, "Caller must be owner");
		let callback = contract_proxy!(self, &self.get_oracle_address(), OracleInterface);
		let callback_address = self.get_sc_address();
		let callback_method = BoxedBytes::from_concat(&[b"reply"]);
		let nonce = self.get_block_nonce();
		let data = BoxedBytes::empty();
		callback.request(callback_address, callback_method, nonce, data);
		Ok(())
	}

	#[callback]
	fn reply(&self, nonce : u64, answer: BoxedBytes) -> SCResult<()> {
		require!(self.get_caller() == self.get_oracle_address(), "Only oracle can reply");
		self.set_client_data(Some(ClientData {
			nonce,
			answer,
		}));
		Ok(())
	}
}
