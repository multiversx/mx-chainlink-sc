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

pub fn format_biguint<BigUint: BigUintApi>(number: &BigUint) -> String {
    let mut nr = number.clone();
    let radix = BigUint::from(10u32);
    let mut result = Vec::new();

    loop {
        let last_digit = nr.clone() % radix.clone();
        nr = nr / radix.clone();

        let digit = *last_digit.to_bytes_be().get(0).unwrap_or(&0) as u8;
        result.push(char::from('0' as u8 + digit));
        if nr == 0 {
            break;
        }
    }
    result.into_iter().rev().collect()
}

fn token_to_string(token_identifier: &TokenIdentifier) -> Result<String, String> {
    String::from_utf8(token_identifier.as_name().into()).map_err(|_| "Invalid token name".into())
}

pub fn format_fixed_precision<BigUint: BigUintApi>(number: &BigUint, decimals: usize) -> String {
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
    ) -> Result<bool, String> {
        let tokens: Vec<&str> = description.split("/").collect();
        if tokens.len() != 2 {
            return Result::Err("Invalid aggregator description format (expected 2 tokens)".into());
        }
        if tokens[0].as_bytes() == source_token.as_esdt_identifier()
            && tokens[1].as_bytes() == target_token.as_esdt_identifier()
        {
            return Result::Ok(false);
        }
        if tokens[0].as_bytes() == target_token.as_esdt_identifier()
            && tokens[1].as_bytes() == source_token.as_esdt_identifier()
        {
            return Result::Ok(true);
        }
        Result::Err("Exchange between chosen token types not supported.".into())
    }

    fn convert(
        &self,
        amount: &BigUint,
        source_token: &TokenIdentifier,
        target_token: &TokenIdentifier,
        multiplier: &BigUint,
        divisor: &BigUint,
        precision_factor: &BigUint,
        decimals: usize,
    ) -> Result<(BigUint, String), String> {
        if divisor == &BigUint::zero() {
            return Result::Err("Convert - dividing by 0".into());
        }
        let converted_amount = amount * multiplier / divisor.clone();
        let rate = multiplier * precision_factor / divisor.clone();
        let message = self.conversion_message(
            amount,
            source_token,
            &rate,
            decimals,
            &converted_amount,
            target_token,
        )?;
        Result::Ok((converted_amount, message))
    }

    fn get_converted_sum(
        &self,
        payment: &BigUint,
        source_token: &TokenIdentifier,
        target_token: &TokenIdentifier,
        exchange_rate: &BigUint,
        decimals: usize,
        reverse_exchange: bool,
    ) -> Result<(BigUint, String), String> {
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

    fn try_convert(
        &self,
        result: AsyncCallResult<Round<BigUint>>,
        payment: &BigUint,
        source_token: &TokenIdentifier,
        target_token: &TokenIdentifier,
    ) -> Result<(BigUint, String), String> {
        match result {
            AsyncCallResult::Ok(round) => {
                let reverse_exchange =
                    self.check_aggregator_tokens(round.description, source_token, target_token)?;
                let (converted_amount, conversion_message) = self.get_converted_sum(
                    payment,
                    source_token,
                    target_token,
                    &round.answer,
                    round.decimals as usize,
                    reverse_exchange,
                )?;
                match self.checked_decrease_balance(target_token, &converted_amount) {
                    Result::Err(error) => {
                        let error_message = String::from_utf8_lossy(error.as_bytes());
                        Result::Err(format!("{} ({})", error_message, conversion_message))
                    }
                    Result::Ok(()) => Result::Ok((converted_amount, conversion_message)),
                }
            }
            AsyncCallResult::Err(error) => {
                self.checked_decrease_balance(source_token, &payment)?;
                let error_message = format!(
                    "Error when getting the price feed from the aggregator: {}",
                    String::from_utf8_lossy(error.err_msg.as_ref())
                );
                Result::Err(error_message)
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
            Result::Ok((converted_payment, conversion_message)) => {
                let message = format!("exchange succesful ({})", conversion_message);
                self.send().direct(
                    &caller,
                    &target_token,
                    &converted_payment,
                    message.as_bytes(),
                );
            }
            Result::Err(error) => {
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

    fn checked_decrease_balance(
        &self,
        token_identifier: &TokenIdentifier,
        amount: &BigUint,
    ) -> Result<(), String> {
        match self.balance().get(&token_identifier) {
            Some(balance) => {
                if &balance < amount {
                    Result::Err(format!(
                        "Insufficient balance: only {} of {} available",
                        format_biguint(&balance),
                        token_to_string(token_identifier)?
                    ))
                } else {
                    self.decrease_balance(token_identifier, amount);
                    Result::Ok(())
                }
            }
            None => Result::Err(format!(
                "No {} tokens are available",
                token_to_string(token_identifier)?
            )),
        }
    }

    fn decrease_balance(&self, token_identifier: &TokenIdentifier, amount: &BigUint) {
        let mut balance = self
            .balance()
            .get(&token_identifier)
            .unwrap_or_else(|| 0u32.into());
        balance -= amount;
        self.balance().insert(token_identifier.clone(), balance);
    }

    fn conversion_message(
        &self,
        payment: &BigUint,
        source_token: &TokenIdentifier,
        rate: &BigUint,
        rate_precision: usize,
        converted_token: &BigUint,
        target_token: &TokenIdentifier,
    ) -> Result<String, String> {
        Result::Ok(format!(
            "conversion from {} of {}, using exchange rate {}, results in {} of {}",
            format_biguint(payment),
            token_to_string(source_token)?,
            format_fixed_precision(rate, rate_precision),
            format_biguint(converted_token),
            token_to_string(target_token)?
        ))
    }

    #[storage_mapper("aggregator")]
    fn aggregator(&self) -> SingleValueMapper<Self::Storage, Address>;

    #[storage_mapper("balance")]
    fn balance(&self) -> MapMapper<Self::Storage, TokenIdentifier, BigUint>;
}
