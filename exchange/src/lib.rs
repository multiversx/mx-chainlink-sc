#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();
use elrond_wasm::String;

extern crate aggregator;
use crate::aggregator::aggregator_interface::{
    AggregatorInterface, AggregatorInterfaceProxy, Round,
};

extern crate alloc;
use alloc::format;

#[derive(TopEncode, TopDecode)]
pub enum EsdtOperation<BigUint: BigUintApi> {
    None,
    Issue,
    Mint(BigUint), // amount minted
}

#[elrond_wasm_derive::contract(EgldEsdtExchangeImpl)]
pub trait EgldEsdtExchange {
    #[init]
    fn init(&self, aggregator: Address) {
        self.aggregator().set(aggregator);
    }

    #[payable("*")]
    #[endpoint]
    fn deposit(
        &self,
        #[payment] payment: BigUint,
        #[payment_token] payment_token: TokenIdentifier,
    ) -> SCResult<()> {
        only_owner!(self, "Only the owner can deposit tokens");
        self.increase_balance(&payment_token, &payment);
        Ok(())
    }

    #[payable("*")]
    #[endpoint]
    fn exchange(
        &self,
        #[payment] payment: BigUint,
        #[payment_token] source_token: TokenIdentifier,
        target_token: TokenIdentifier,
    ) -> SCResult<AsyncCall<BigUint>> {
        require!(payment > 0, "Payment must be more than 0");
        require!(
            self.balance().contains_key(&source_token),
            "ESDT provided as payment not supported by the exchange"
        );
        require!(
            self.balance().contains_key(&target_token),
            "Target ESDT token not supported by the exchange"
        );
        self.increase_balance(&source_token, &payment);

        Ok(
            contract_call!(self, self.aggregator().get(), AggregatorInterfaceProxy)
                .latest_round_data()
                .async_call()
                .with_callback(self.callbacks().finalize_exchange(
                    payment,
                    source_token,
                    target_token,
                )),
        )
    }

    fn check_aggregator_tokens(
        &self,
        description: String,
        source_token: &TokenIdentifier,
        target_token: &TokenIdentifier,
    ) -> SCResult<bool> {
        let tokens: Vec<&str> = description.split("/").collect();
        require!(
            tokens.len() == 2,
            "Invalid aggregator description format (expected 2 tokens)"
        );
        if tokens[0].as_bytes() == source_token.as_slice()
            && tokens[1].as_bytes() == target_token.as_slice()
        {
            return Ok(false);
        }
        if tokens[0].as_bytes() == target_token.as_slice()
            && tokens[1].as_bytes() == source_token.as_slice()
        {
            return Ok(true);
        }
        sc_error!("Exchange between chosen token types not supported.")
    }

    fn get_converted_sum(
        &self,
        payment: &BigUint,
        exchange_rate: &BigUint,
        decimals: u8,
        reverse_exchange: bool,
    ) -> BigUint {
        let factor = BigUint::from(10u64.pow(decimals as u32));
        if reverse_exchange {
            payment * &factor / exchange_rate.clone()
        } else {
            payment * exchange_rate / factor
        }
    }

    fn try_convert(
        &self,
        result: AsyncCallResult<Round<BigUint>>,
        payment: &BigUint,
        source_token: &TokenIdentifier,
        target_token: &TokenIdentifier,
    ) -> SCResult<BigUint> {
        match result {
            AsyncCallResult::Ok(round) => {
                let reverse_exchange = sc_try!(self.check_aggregator_tokens(
                    round.description,
                    source_token,
                    target_token
                ));
                let converted_payment = self.get_converted_sum(
                    payment,
                    &round.answer,
                    round.decimals,
                    reverse_exchange,
                );
                sc_try!(self.decrease_balance(target_token, &converted_payment));
                Ok(converted_payment)
            }
            AsyncCallResult::Err(error) => {
                sc_try!(self.decrease_balance(source_token, &payment));
                let error_message = format!(
                    "Error when getting the price feed from the aggregator: {:?}",
                    error.err_msg.as_ref()
                );
                sc_error!(error_message)
            }
        }
    }

    #[callback]
    fn finalize_exchange(
        &self,
        #[call_result] result: AsyncCallResult<Round<BigUint>>,
        payment: BigUint,
        source_token: TokenIdentifier,
        target_token: TokenIdentifier,
    ) {
        match self.try_convert(result, &payment, &source_token, &target_token) {
            Ok(converted_payment) => {
                self.send().direct(
                    &self.get_caller(),
                    &target_token,
                    &converted_payment,
                    b"exchange",
                );
            }
            Err(error) => {
                let message = format!("refund ({:?})", error.as_bytes());
                self.send().direct(
                    &self.get_caller(),
                    &source_token,
                    &payment,
                    message.as_bytes(),
                );
            }
        }
    }

    fn increase_balance(&self, token_identifier: &TokenIdentifier, amount: &BigUint) {
        let mut balance = self
            .balance()
            .get(&token_identifier)
            .unwrap_or_else(|| 0u32.into());
        balance += amount;
        self.balance().insert(token_identifier.clone(), balance);
    }

    fn decrease_balance(
        &self,
        token_identifier: &TokenIdentifier,
        amount: &BigUint,
    ) -> SCResult<()> {
        if let Some(balance) = self.balance().get(&token_identifier) {
            if &balance < amount {
                self.balance()
                    .insert(token_identifier.clone(), balance - amount.clone());
                Ok(())
            } else {
                sc_error!("Insufficient balance")
            }
        } else {
            sc_error!("Token not found")
        }
    }

    #[storage_mapper("aggregator")]
    fn aggregator(&self) -> GetterSetterMapper<Self::Storage, Address>;

    #[storage_mapper("balance")]
    fn balance(&self) -> MapMapper<Self::Storage, TokenIdentifier, BigUint>;
}
