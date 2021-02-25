#![no_std]

use elrond_wasm::HexCallDataSerializer;

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

extern crate aggregator;
use aggregator::*;

extern crate alloc;
use alloc::format;

// erd1qqqqqqqqqqqqqqqpqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqzllls8a5w6u
const ESDT_SYSTEM_SC_ADDRESS_ARRAY: [u8; 32] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0xff, 0xff,
];

const ESDT_ISSUE_COST: u64 = 5000000000000000000; // 5 eGLD

const ESDT_ISSUE_STRING: &[u8] = b"issue";
const ESDT_MINT_STRING: &[u8] = b"mint";

const EGLD_DECIMALS: u8 = 18;

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
        self.aggregator_address().set(aggregator);
    }

    // endpoints - owner-only

    #[payable("EGLD")]
    #[endpoint(performWrappedEgldIssue)]
    fn perform_wrapped_egld_issue(
        &self,
        token_display_name: BoxedBytes,
        token_ticker: BoxedBytes,
        initial_supply: BigUint,
        #[payment] payment: BigUint,
    ) -> SCResult<()> {
        only_owner!(self, "only owner may call this function");

        require!(
            self.is_empty_wrapped_egld_token_identifier(),
            "wrapped egld was already issued"
        );
        require!(
            payment == BigUint::from(ESDT_ISSUE_COST),
            "Wrong payment, should pay exactly 5 eGLD for ESDT token issue"
        );

        self.issue_esdt_token(
            &token_display_name,
            &token_ticker,
            &initial_supply,
            EGLD_DECIMALS,
        );

        Ok(())
    }

    #[endpoint(mintWrappedEgld)]
    fn mint_wrapped_egld(&self, amount: BigUint) -> SCResult<()> {
        only_owner!(self, "only owner may call this function");

        require!(
            !self.is_empty_wrapped_egld_token_identifier(),
            "Wrapped eGLD was not issued yet"
        );

        self.mint_esdt_token(&self.get_wrapped_egld_token_identifier(), &amount);

        Ok(())
    }

    // endpoints

    #[payable("EGLD")]
    #[endpoint]
    fn sell_egld(&self, #[payment] payment: BigUint) -> SCResult<AsyncCall<BigUint>> {
        require!(payment > 0, "Payment must be more than 0");
        require!(
            !self.is_empty_wrapped_egld_token_identifier(),
            "ESDT was not issued yet"
        );

        Ok(contract_call!(
            self,
            self.aggregator_address().get(),
            AggregatorInterfaceProxy
        )
        .latest_round_data()
        .async_call()
        .with_callback(self.callbacks().sell_rate_received(payment)))
    }

    fn try_convert_sell(
        &self,
        result: AsyncCallResult<aggregator::Round<BigUint>>,
        payment: &BigUint,
    ) -> SCResult<BigUint> {
        match result {
            AsyncCallResult::Ok(round) => {
                let converted_payment =
                    payment * &round.answer / BigUint::from(10u64.pow(round.decimals as u32));

                let wrapped_egld_left = self.get_wrapped_egld_remaining();
                require!(
                    wrapped_egld_left > converted_payment,
                    "Contract does not have enough ESDT. Please try again once more is minted."
                );

                self.substract_total_wrapped_egld(&payment);

                Ok(converted_payment)
            }
            AsyncCallResult::Err(error) => {
                let error_message = format!(
                    "error when getting the price feed from the aggregator: {:?}",
                    error.err_msg.as_ref()
                );
                sc_error!(error_message)
            }
        }
    }

    #[callback]
    fn sell_rate_received(
        &self,
        #[call_result] result: AsyncCallResult<aggregator::Round<BigUint>>,
        payment: BigUint,
    ) {
        match self.try_convert_sell(result, &payment) {
            Ok(converted_payment) => {
                self.send().direct_esdt(
                    &self.get_caller(),
                    self.get_wrapped_egld_token_identifier().as_slice(),
                    &converted_payment,
                    b"wrapping",
                );
            }
            Err(error) => {
                let message = format!("refund ({:?})", error.as_bytes());
                self.send()
                    .direct_egld(&self.get_caller(), &payment, message.as_bytes());
            }
        }
    }

    #[payable("*")]
    #[endpoint]
    fn buy_egld(
        &self,
        #[payment] wrapped_egld_payment: BigUint,
        #[payment_token] token_identifier: TokenIdentifier,
    ) -> SCResult<AsyncCall<BigUint>> {
        require!(
            !self.is_empty_wrapped_egld_token_identifier(),
            "Wrapped eGLD was not issued yet"
        );
        require!(token_identifier.is_esdt(), "Only ESDT tokens accepted");

        let wrapped_egld_token_identifier = self.get_wrapped_egld_token_identifier();

        require!(
            token_identifier == wrapped_egld_token_identifier,
            "Wrong esdt token"
        );

        require!(wrapped_egld_payment > 0, "Must pay more than 0 tokens!");

        Ok(contract_call!(
            self,
            self.aggregator_address().get(),
            AggregatorInterfaceProxy
        )
        .latest_round_data()
        .async_call()
        .with_callback(
            self.callbacks()
                .buy_rate_received(wrapped_egld_payment, token_identifier),
        ))
    }

    fn try_convert_buy(
        &self,
        result: AsyncCallResult<aggregator::Round<BigUint>>,
        payment: &BigUint,
    ) -> SCResult<BigUint> {
        match result {
            AsyncCallResult::Ok(round) => {
                let converted_payment =
                    payment * &BigUint::from(10u64.pow(round.decimals as u32)) / round.answer;

                require!(
                    converted_payment <= self.get_sc_balance(),
                    "Contract does not have enough funds"
                );

                self.add_total_wrapped_egld(&converted_payment);

                Ok(converted_payment)
            }
            AsyncCallResult::Err(error) => {
                let error_message = format!(
                    "error when getting the price feed from the aggregator: {:?}",
                    error.err_msg.as_ref()
                );
                sc_error!(error_message)
            }
        }
    }

    #[callback]
    fn buy_rate_received(
        &self,
        #[call_result] result: AsyncCallResult<aggregator::Round<BigUint>>,
        token_payment: BigUint,
        token_identifier: TokenIdentifier,
    ) {
        match self.try_convert_buy(result, &token_payment) {
            Ok(converted_payment) => {
                self.send()
                    .direct_egld(&self.get_caller(), &converted_payment, b"unwrapping");
            }
            Err(error) => {
                let message = format!("refund ({:?})", error.as_bytes());
                self.send().direct_esdt(
                    &self.get_caller(),
                    &token_identifier.as_slice(),
                    &token_payment,
                    message.as_bytes(),
                );
            }
        }
    }

    #[view(getLockedEgldBalance)]
    fn get_locked_egld_balance() -> BigUint {
        self.get_sc_balance()
    }

    // private

    fn add_total_wrapped_egld(&self, amount: &BigUint) {
        let mut total_wrapped = self.get_wrapped_egld_remaining();
        total_wrapped += amount;
        self.set_wrapped_egld_remaining(&total_wrapped);
    }

    fn substract_total_wrapped_egld(&self, amount: &BigUint) {
        let mut total_wrapped = self.get_wrapped_egld_remaining();
        total_wrapped -= amount;
        self.set_wrapped_egld_remaining(&total_wrapped);
    }

    fn issue_esdt_token(
        &self,
        token_display_name: &BoxedBytes,
        token_ticker: &BoxedBytes,
        initial_supply: &BigUint,
        num_decimals: u8,
    ) {
        let mut serializer = HexCallDataSerializer::new(ESDT_ISSUE_STRING);

        serializer.push_argument_bytes(token_display_name.as_slice());
        serializer.push_argument_bytes(token_ticker.as_slice());
        serializer.push_argument_bytes(&initial_supply.to_bytes_be());
        serializer.push_argument_bytes(&[num_decimals]);

        serializer.push_argument_bytes(&b"canFreeze"[..]);
        serializer.push_argument_bytes(&b"false"[..]);

        serializer.push_argument_bytes(&b"canWipe"[..]);
        serializer.push_argument_bytes(&b"false"[..]);

        serializer.push_argument_bytes(&b"canPause"[..]);
        serializer.push_argument_bytes(&b"false"[..]);

        serializer.push_argument_bytes(&b"canMint"[..]);
        serializer.push_argument_bytes(&b"true"[..]);

        serializer.push_argument_bytes(&b"canBurn"[..]);
        serializer.push_argument_bytes(&b"true"[..]);

        serializer.push_argument_bytes(&b"canChangeOwner"[..]);
        serializer.push_argument_bytes(&b"false"[..]);

        serializer.push_argument_bytes(&b"canUpgrade"[..]);
        serializer.push_argument_bytes(&b"false"[..]);

        // save data for callback
        self.set_temporary_storage_esdt_operation(&self.get_tx_hash(), &EsdtOperation::Issue);

        self.send().async_call_raw(
            &Address::from(ESDT_SYSTEM_SC_ADDRESS_ARRAY),
            &BigUint::from(ESDT_ISSUE_COST),
            serializer.as_slice(),
        );
    }

    fn mint_esdt_token(&self, token_identifier: &TokenIdentifier, amount: &BigUint) {
        let mut serializer = HexCallDataSerializer::new(ESDT_MINT_STRING);
        serializer.push_argument_bytes(token_identifier.as_slice());
        serializer.push_argument_bytes(&amount.to_bytes_be());

        // save data for callback
        self.set_temporary_storage_esdt_operation(
            &self.get_tx_hash(),
            &EsdtOperation::Mint(amount.clone()),
        );

        self.send().async_call_raw(
            &Address::from(ESDT_SYSTEM_SC_ADDRESS_ARRAY),
            &BigUint::zero(),
            serializer.as_slice(),
        );
    }

    // callbacks

    #[callback_raw]
    fn callback_raw(&self, result: Vec<Vec<u8>>) {
        // "0" is serialized as "nothing", so len == 0 for the first item is error code of 0, which means success
        let success = result[0].len() == 0;
        let original_tx_hash = self.get_tx_hash();

        let esdt_operation = self.get_temporary_storage_esdt_operation(&original_tx_hash);
        match esdt_operation {
            EsdtOperation::None => return,
            EsdtOperation::Issue => self.perform_esdt_issue_callback(success),
            EsdtOperation::Mint(amount) => self.perform_esdt_mint_callback(success, &amount),
        };

        self.clear_temporary_storage_esdt_operation(&original_tx_hash);
    }

    fn perform_esdt_issue_callback(&self, success: bool) {
        // callback is called with ESDTTransfer of the newly issued token, with the amount requested,
        // so we can get the token identifier and amount from the call data
        let token_identifier = self.call_value().token();
        let initial_supply = self.call_value().esdt_value();

        if success {
            self.set_wrapped_egld_remaining(&initial_supply);
            self.set_wrapped_egld_token_identifier(&token_identifier);
        }

        // nothing to do in case of error
    }

    fn perform_esdt_mint_callback(&self, success: bool, amount: &BigUint) {
        if success {
            self.add_total_wrapped_egld(amount);
        }

        // nothing to do in case of error
    }

    // storage

    // 1 eGLD = 1 wrapped eGLD, and they are interchangeable through this contract

    #[view(getWrappedEgldTokenIdentifier)]
    #[storage_get("wrappedEgldTokenIdentifier")]
    fn get_wrapped_egld_token_identifier(&self) -> TokenIdentifier;

    #[storage_set("wrappedEgldTokenIdentifier")]
    fn set_wrapped_egld_token_identifier(&self, token_identifier: &TokenIdentifier);

    #[storage_is_empty("wrappedEgldTokenIdentifier")]
    fn is_empty_wrapped_egld_token_identifier(&self) -> bool;

    #[view(getWrappedEgldRemaining)]
    #[storage_get("wrappedEgldRemaining")]
    fn get_wrapped_egld_remaining(&self) -> BigUint;

    #[storage_set("wrappedEgldRemaining")]
    fn set_wrapped_egld_remaining(&self, wrapped_egld_remaining: &BigUint);

    // temporary storage for ESDT operations. Used in callback.

    #[storage_get("temporaryStorageEsdtOperation")]
    fn get_temporary_storage_esdt_operation(
        &self,
        original_tx_hash: &H256,
    ) -> EsdtOperation<BigUint>;

    #[storage_set("temporaryStorageEsdtOperation")]
    fn set_temporary_storage_esdt_operation(
        &self,
        original_tx_hash: &H256,
        esdt_operation: &EsdtOperation<BigUint>,
    );

    #[storage_clear("temporaryStorageEsdtOperation")]
    fn clear_temporary_storage_esdt_operation(&self, original_tx_hash: &H256);

    #[storage_mapper("aggregator_address")]
    fn aggregator_address(&self) -> GetterSetterMapper<Self::Storage, Address>;
}
