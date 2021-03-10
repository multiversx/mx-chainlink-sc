#![no_std]
#![feature(assoc_char_funcs)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();
use elrond_wasm::String;

extern crate aggregator;
use crate::aggregator::aggregator_interface::{
    AggregatorInterface, AggregatorInterfaceProxy, Round,
};

extern crate alloc;
use alloc::format;

// The following comment applies to this file in general.
//
// while I definitely understand the temptation to have nicely formatted messages,
// the wasm bloat is insane. This is fine for debugging, but in my opinion,
// this should never make it into the final product

fn format_biguint<BigUint: BigUintApi>(number: &BigUint) -> String {
    // "x" is not a good variable name. At the very least, rename to "nr"
    let mut x = number.clone();
    // to prevent doing conversions over and over through .into()
    // rename to radix_uint and have a radix_biguint = BigUint::from(10u32) variable as well
    let radix = 10u32;
    let mut result = Vec::new();

    loop {
        // rename "m" to "remainder" or something similar
        let m = x.clone() % radix.into();
        x = x / radix.into();

        // will panic if you use a bad radix (< 2 or > 36).
        let digit = *m.to_bytes_be().get(0).unwrap_or(&0) as u32;
        
        // char is always 4 bytes, so this could be optimized to use (b'0' + digit) instead, which would give you an u8,
        // which you could then use to create the String (String::from_ut8 iirc)
        result.push(char::from_digit(digit, radix).unwrap());
        if x == 0 {
            break;
        }
    }
    result.into_iter().rev().collect()
}

fn token_to_string(token_identifier: &TokenIdentifier) -> String {
    // There is no need to use _lossy version. Token identifier uses u8 slices. 
    // All ASCII characters are valid UTF-8
    // Invalid characters (> 127) would mean the token identifier itself is invalid
    String::from_utf8_lossy(token_identifier.as_name()).into()
}

fn format_fixed_precision<BigUint: BigUintApi>(number: &BigUint, decimals: usize) -> String {
    let padded_number = format!("{:0>width$}", format_biguint(number), width = decimals + 1);
    let digits_before_dot = padded_number.len() - decimals;
    let left = padded_number
        .chars()
        .take(digits_before_dot)
        .collect::<String>();
    let right = padded_number
        .chars()
        .skip(digits_before_dot)
        .collect::<String>();
    format!("{}.{}", left, right)
}

#[elrond_wasm_derive::contract(EgldEsdtExchangeImpl)]
pub trait EgldEsdtExchange {
    #[init]
    fn init(&self, aggregator: Address) {
        self.aggregator().set(&aggregator);
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
                    self.get_caller(),
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
        if tokens[0].as_bytes() == source_token.as_esdt_identifier()
            && tokens[1].as_bytes() == target_token.as_esdt_identifier()
        {
            return Ok(false);
        }
        if tokens[0].as_bytes() == target_token.as_esdt_identifier()
            && tokens[1].as_bytes() == source_token.as_esdt_identifier()
        {
            return Ok(true);
        }
        sc_error!("Exchange between chosen token types not supported.")
    }

    // Optional: rename "source_amount" to "amount"
    fn convert(
        &self,
        source_amount: &BigUint,
        source_token: &TokenIdentifier,
        target_token: &TokenIdentifier,
        multiplier: &BigUint,
        divisor: &BigUint,
        precision_factor: &BigUint,
        decimals: usize,
    ) -> SCResult<(BigUint, String)> {
        require!(divisor > &BigUint::from(0u32), "Convert - dividing by 0");
        let converted_amount = source_amount * multiplier / divisor.clone();
        let rate = multiplier * precision_factor / divisor.clone();
        let message = self.conversion_message(
            source_amount,
            source_token,
            &rate,
            decimals,
            &converted_amount,
            target_token,
        );
        Ok((converted_amount, message))
    }

    fn get_converted_sum(
        &self,
        payment: &BigUint,
        source_token: &TokenIdentifier,
        target_token: &TokenIdentifier,
        exchange_rate: &BigUint,
        decimals: usize,
        reverse_exchange: bool,
    ) -> SCResult<(BigUint, String)> {
        let precision_factor = BigUint::from(10u64.pow(decimals as u32));
        if !reverse_exchange {
            self.convert(
                payment,
                source_token,
                target_token,
                exchange_rate,
                &precision_factor,
                &precision_factor,
                decimals,
            )
        } else {
            self.convert(
                payment,
                source_token,
                target_token,
                &precision_factor,
                exchange_rate,
                &precision_factor,
                decimals,
            )
        }
    }

    // SCResult should be used strictly for returning data to the caller.
    // This function is used only in the callback, so
    // I suggest using core::Result in this case
    // Same with check_aggregator_tokens and get_converted_sum
    fn try_convert(
        &self,
        result: AsyncCallResult<Round<BigUint>>,
        payment: &BigUint,
        source_token: &TokenIdentifier,
        target_token: &TokenIdentifier,
    ) -> SCResult<(BigUint, String)> {
        match result {
            AsyncCallResult::Ok(round) => {
                let reverse_exchange = sc_try!(self.check_aggregator_tokens(
                    round.description,
                    source_token,
                    target_token
                ));
                let (converted_amount, conversion_message) = sc_try!(self.get_converted_sum(
                    payment,
                    source_token,
                    target_token,
                    &round.answer,
                    round.decimals as usize,
                    reverse_exchange,
                ));

                // use "match" instead of "if"
                if let SCResult::Err(error) = self.decrease_balance(target_token, &converted_amount)
                {
                    let error_message = String::from_utf8_lossy(error.as_bytes());
                    return sc_error!(format!("{} ({})", error_message, conversion_message));
                }
                Ok((converted_amount, conversion_message))
            }
            AsyncCallResult::Err(error) => {
                sc_try!(self.decrease_balance(source_token, &payment));
                let error_message = format!(
                    "Error when getting the price feed from the aggregator: {}",
                    String::from_utf8_lossy(error.err_msg.as_ref())
                );
                sc_error!(error_message)
            }
        }
    }

    #[callback]
    fn finalize_exchange(
        &self,
        #[call_result] result: AsyncCallResult<Round<BigUint>>,
        caller: Address,
        payment: BigUint,
        source_token: TokenIdentifier,
        target_token: TokenIdentifier,
    ) {
        match self.try_convert(result, &payment, &source_token, &target_token) {
            Ok((converted_payment, conversion_message)) => {
                let message = format!("exchange succesful ({})", conversion_message);
                self.send().direct(
                    &caller,
                    &target_token,
                    &converted_payment,
                    message.as_bytes(),
                );
            }
            Err(error) => {
                let message = format!("refund ({})", String::from_utf8_lossy(error.as_bytes()));
                self.send()
                    .direct(&caller, &source_token, &payment, message.as_bytes());
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

    // For consistency with the increase_balance function, do all the checking in the caller
    // and let this function simply update the storage value, then don't return anything
    fn decrease_balance(
        &self,
        token_identifier: &TokenIdentifier,
        amount: &BigUint,
    ) -> SCResult<()> {
        if let Some(balance) = self.balance().get(&token_identifier) {
            if &balance >= amount {
                self.balance()
                    .insert(token_identifier.clone(), balance - amount.clone());
                Ok(())
            } else {
                sc_error!(format!(
                    "Insufficient balance: only {} of {} available",
                    format_biguint(&balance),
                    token_to_string(token_identifier)
                ))
            }
        } else {
            sc_error!(format!(
                "No {} tokens are available",
                token_to_string(token_identifier)
            ))
        }
    }

    fn conversion_message(
        &self,
        payment: &BigUint,
        source_token: &TokenIdentifier,
        rate: &BigUint,
        rate_precision: usize,
        converted_token: &BigUint,
        target_token: &TokenIdentifier,
    ) -> String {
        format!(
            "conversion from {} of {}, using exchange rate {}, results in {} of {}",
            format_biguint(payment),
            token_to_string(source_token),
            format_fixed_precision(rate, rate_precision),
            format_biguint(converted_token),
            token_to_string(target_token)
        )
    }

    #[storage_mapper("aggregator")]
    fn aggregator(&self) -> SingleValueMapper<Self::Storage, Address>;

    #[storage_mapper("balance")]
    fn balance(&self) -> MapMapper<Self::Storage, TokenIdentifier, BigUint>;
}
