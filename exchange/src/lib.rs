#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

extern crate aggregator;

use crate::aggregator::aggregator_interface::Round;

#[macro_use]
extern crate alloc;

pub fn format_biguint<M: ManagedTypeApi>(number: &BigUint<M>) -> Vec<u8> {
    let mut nr = number.clone();
    let radix: BigUint<M> = 10u32.managed_into(number.type_manager());
    let mut result = Vec::new();

    loop {
        let last_digit = &nr.clone() % &radix;
        nr /= &radix;

        let last_digit_bytes = last_digit.to_bytes_be();
        let digit = *last_digit_bytes.as_slice().get(0).unwrap_or(&0) as u8;
        result.push('0' as u8 + digit);
        if nr == 0u32 {
            break;
        }
    }
    result.into_iter().rev().collect()
}

pub fn format_fixed_precision<M: ManagedTypeApi>(number: &BigUint<M>, decimals: usize) -> Vec<u8> {
    let formatted_number = format_biguint(number);
    let padding_length = (decimals + 1)
        .checked_sub(formatted_number.len())
        .unwrap_or_default();
    let padding: Vec<u8> = vec!['0' as u8; padding_length];
    let padded_number = BoxedBytes::from_concat(&[padding.as_slice(), formatted_number.as_slice()]);
    let digits_before_dot = padded_number.len() - decimals;

    let left = padded_number.as_slice().iter().take(digits_before_dot);
    let dot = core::iter::once(&('.' as u8));
    let right = padded_number.as_slice().iter().skip(digits_before_dot);
    left.chain(dot).chain(right).cloned().collect()
}

#[elrond_wasm::derive::contract]
pub trait EgldEsdtExchange {
    #[init]
    fn init(&self, aggregator: ManagedAddress) {
        self.aggregator().set(&aggregator);
    }

    #[payable("*")]
    #[endpoint(deposit)]
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
    #[endpoint(exchange)]
    fn exchange(
        &self,
        #[payment] payment: BigUint,
        #[payment_token] source_token: TokenIdentifier,
        target_token: TokenIdentifier,
    ) -> SCResult<AsyncCall> {
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

        Ok(self
            .aggregator_interface_proxy(self.aggregator().get())
            .latest_round_data()
            .async_call()
            .with_callback(self.callbacks().finalize_exchange(
                self.blockchain().get_caller(),
                payment,
                source_token,
                target_token,
            )))
    }

    fn check_aggregator_tokens(
        &self,
        description: BoxedBytes,
        source_token: &TokenIdentifier,
        target_token: &TokenIdentifier,
    ) -> Result<bool, BoxedBytes> {
        let delimiter_position = description
            .as_slice()
            .iter()
            .position(|item| *item == '/' as u8)
            .ok_or(BoxedBytes::from(
                "Invalid aggregator description format (expected 2 tokens)".as_bytes(),
            ))?;
        let (first, second) = description.split(delimiter_position);
        let first_token: &TokenIdentifier = &first.as_slice().managed_into();
        let second_token: &TokenIdentifier = &second.as_slice().managed_into();
        if first_token == source_token && second_token == target_token {
            return Result::Ok(false);
        }
        if first_token == target_token && second_token == source_token {
            return Result::Ok(true);
        }
        Result::Err(
            "Exchange between chosen token types not supported."
                .as_bytes()
                .into(),
        )
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
    ) -> Result<(BigUint, BoxedBytes), BoxedBytes> {
        if divisor == &BigUint::zero() {
            return Result::Err("Convert - dividing by 0".as_bytes().into());
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
    ) -> Result<(BigUint, BoxedBytes), BoxedBytes> {
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
        result: AsyncCallResult<OptionalArg<Round<Self::Api>>>,
        payment: &BigUint,
        source_token: &TokenIdentifier,
        target_token: &TokenIdentifier,
    ) -> Result<(BigUint, BoxedBytes), BoxedBytes> {
        match result {
            AsyncCallResult::Ok(optional_result_round) => {
                let option_round = match optional_result_round {
                    OptionalArg::Some(round) => Some(round),
                    OptionalArg::None => None,
                };
                let error_message: BoxedBytes = b"no round data"[..].into();
                let round = option_round.ok_or(error_message)?;
                let error_message: BoxedBytes = b"no aggregator data"[..].into();
                let submission = round.answer.ok_or(error_message)?;
                if submission.values.len() != 1 {
                    let error_message: BoxedBytes = b"invalid aggregator data format"[..].into();
                    return Result::Err(error_message);
                }
                let exchange_rate = &submission.values[0];
                let reverse_exchange =
                    self.check_aggregator_tokens(round.description, source_token, target_token)?;
                let (converted_amount, conversion_message) = self.get_converted_sum(
                    payment,
                    source_token,
                    target_token,
                    exchange_rate,
                    round.decimals as usize,
                    reverse_exchange,
                )?;
                match self.checked_decrease_balance(target_token, &converted_amount) {
                    Result::Err(error) => Result::Err(BoxedBytes::from_concat(&[
                        error.as_slice(),
                        b" (",
                        conversion_message.as_slice(),
                        b")",
                    ])),
                    Result::Ok(()) => Result::Ok((converted_amount, conversion_message)),
                }
            }
            AsyncCallResult::Err(error) => {
                self.checked_decrease_balance(source_token, &payment)?;
                Result::Err(BoxedBytes::from_concat(&[
                    b"Error when getting the price feed from the aggregator: ",
                    error.err_msg.as_slice(),
                ]))
            }
        }
    }

    #[callback]
    fn finalize_exchange(
        &self,
        #[call_result] result: AsyncCallResult<OptionalArg<Round<Self::Api>>>,
        caller: ManagedAddress,
        payment: BigUint,
        source_token: TokenIdentifier,
        target_token: TokenIdentifier,
    ) {
        match self.try_convert(result, &payment, &source_token, &target_token) {
            Result::Ok((converted_payment, conversion_message)) => {
                let message = BoxedBytes::from_concat(&[
                    b"exchange succesful ",
                    b"(",
                    conversion_message.as_slice(),
                    b")",
                ]);
                self.send().direct(
                    &caller,
                    &target_token,
                    0,
                    &converted_payment,
                    message.as_slice(),
                );
            }
            Result::Err(error) => {
                let message = BoxedBytes::from_concat(&[b"refund (", error.as_slice(), b")"]);
                self.send()
                    .direct(&caller, &source_token, 0, &payment, message.as_slice());
            }
        }
    }

    fn increase_balance(&self, token_identifier: &TokenIdentifier, amount: &BigUint) {
        let mut balance = self
            .balance()
            .get(&token_identifier)
            .unwrap_or_else(|| BigUint::zero());
        balance += amount;
        self.balance().insert(token_identifier.clone(), balance);
    }

    fn checked_decrease_balance(
        &self,
        token_identifier: &TokenIdentifier,
        amount: &BigUint,
    ) -> Result<(), BoxedBytes> {
        match self.balance().get(&token_identifier) {
            Some(balance) => {
                if &balance < amount {
                    Result::Err(BoxedBytes::from_concat(&[
                        b"Insufficient balance: only ",
                        format_biguint(&balance).as_slice(),
                        b" of ",
                        token_identifier.as_name().as_slice(),
                        b" available",
                    ]))
                } else {
                    self.decrease_balance(token_identifier, amount);
                    Result::Ok(())
                }
            }
            None => Result::Err(BoxedBytes::from_concat(&[
                b"No ",
                token_identifier.as_name().as_slice(),
                b" tokens are available",
            ])),
        }
    }

    fn decrease_balance(&self, token_identifier: &TokenIdentifier, amount: &BigUint) {
        let mut balance = self
            .balance()
            .get(&token_identifier)
            .unwrap_or_else(|| BigUint::zero());
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
    ) -> Result<BoxedBytes, BoxedBytes> {
        Result::Ok(BoxedBytes::from_concat(&[
            b"conversion from ",
            format_biguint(payment).as_slice(),
            b" of ",
            source_token.as_name().as_slice(),
            b", using exchange rate ",
            format_fixed_precision(rate, rate_precision).as_slice(),
            b", results in ",
            format_biguint(converted_token).as_slice(),
            b" of ",
            target_token.as_name().as_slice(),
        ]))
    }

    #[storage_mapper("aggregator")]
    fn aggregator(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("balance")]
    fn balance(&self) -> MapMapper<TokenIdentifier, BigUint>;

    #[proxy]
    fn aggregator_interface_proxy(&self, to: ManagedAddress) -> aggregator::Proxy<Self::Api>;
}
