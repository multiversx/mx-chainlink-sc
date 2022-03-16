#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

extern crate aggregator;
use aggregator::aggregator_interface::DescriptionVec;

use crate::aggregator::aggregator_interface::Round;

const MAX_FORMATTED_NUMBER_CHARS: usize = 30;
const MAX_PADDING_LEN: usize = 18;
const MAX_PADDED_NUMBER_CHARS: usize = MAX_FORMATTED_NUMBER_CHARS + MAX_PADDING_LEN;

pub fn format_biguint<M: ManagedTypeApi>(
    mut number: BigUint<M>,
) -> ArrayVec<u8, MAX_FORMATTED_NUMBER_CHARS> {
    let radix = BigUint::from(10u32);
    let mut result = ArrayVec::<u8, MAX_FORMATTED_NUMBER_CHARS>::new();

    loop {
        let last_digit = &number % &radix;
        number /= &radix;

        let digit = last_digit.to_u64().unwrap_or_default() as u8;
        result.push('0' as u8 + digit);

        if number == 0u32 {
            break;
        }
    }

    result.into_iter().rev().collect()
}

pub fn format_fixed_precision<M: ManagedTypeApi>(
    number: BigUint<M>,
    decimals: usize,
) -> ArrayVec<u8, MAX_FORMATTED_NUMBER_CHARS> {
    let formatted_number = format_biguint(number);
    let padding_length = (decimals + 1)
        .checked_sub(formatted_number.len())
        .unwrap_or_default();

    // is there a better way to do this?
    let mut padding = ArrayVec::<u8, MAX_PADDING_LEN>::new();
    for _ in 0..padding_length {
        padding.push(b'0');
    }

    let mut padded_number = ArrayVec::<u8, MAX_PADDED_NUMBER_CHARS>::new();
    padded_number.extend(padding);
    padded_number.extend(formatted_number);

    let digits_before_dot = padded_number.len() - decimals;

    let left = padded_number.as_slice().iter().take(digits_before_dot);
    let dot = core::iter::once(&('.' as u8));
    let right = padded_number.as_slice().iter().skip(digits_before_dot);
    left.chain(dot).chain(right).cloned().collect()
}

#[elrond_wasm::contract]
pub trait EgldEsdtExchange {
    #[init]
    fn init(&self, aggregator: ManagedAddress) {
        self.aggregator().set(&aggregator);
    }

    #[only_owner]
    #[payable("*")]
    #[endpoint(deposit)]
    fn deposit(
        &self,
        #[payment] payment: BigUint,
        #[payment_token] payment_token: TokenIdentifier,
    ) {
        self.increase_balance(&payment_token, &payment);
    }

    #[payable("*")]
    #[endpoint(exchange)]
    fn exchange(
        &self,
        #[payment] payment: BigUint,
        #[payment_token] source_token: TokenIdentifier,
        target_token: TokenIdentifier,
    ) {
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

        self.aggregator_interface_proxy(self.aggregator().get())
            .latest_round_data()
            .async_call()
            .with_callback(self.callbacks().finalize_exchange(
                self.blockchain().get_caller(),
                payment,
                source_token,
                target_token,
            ))
            .call_and_exit();
    }

    fn check_aggregator_tokens(
        &self,
        description: DescriptionVec,
        source_token: &TokenIdentifier,
        target_token: &TokenIdentifier,
    ) -> Result<bool, ManagedBuffer> {
        let delimiter_position = description
            .as_slice()
            .iter()
            .position(|item| *item == '/' as u8)
            .ok_or(ManagedBuffer::from(
                "Invalid aggregator description format (expected 2 tokens)".as_bytes(),
            ))?;
        let (first, second) = description.split_at(delimiter_position);
        let first_token = TokenIdentifier::from(first);
        let second_token = TokenIdentifier::from(second);
        if &first_token == source_token && &second_token == target_token {
            return Result::Ok(false);
        }
        if &first_token == target_token && &second_token == source_token {
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
    ) -> Result<(BigUint, ManagedBuffer), ManagedBuffer> {
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
    ) -> Result<(BigUint, ManagedBuffer), ManagedBuffer> {
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
        result: ManagedAsyncCallResult<OptionalValue<Round<Self::Api>>>,
        payment: &BigUint,
        source_token: &TokenIdentifier,
        target_token: &TokenIdentifier,
    ) -> Result<(BigUint, ManagedBuffer), ManagedBuffer> {
        match result {
            ManagedAsyncCallResult::Ok(optional_result_round) => {
                let option_round = match optional_result_round {
                    OptionalValue::Some(round) => Some(round),
                    OptionalValue::None => None,
                };
                let error_message: ManagedBuffer = b"no round data"[..].into();
                let round = option_round.ok_or(error_message)?;
                let error_message: ManagedBuffer = b"no aggregator data"[..].into();
                let submission = round.answer.ok_or(error_message)?;
                if submission.values.len() != 1 {
                    let error_message: ManagedBuffer = b"invalid aggregator data format"[..].into();
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
                    Result::Err(mut error) => {
                        error.append_bytes(b" (");
                        error.append(&conversion_message);
                        error.append_bytes(b")");

                        Result::Err(error)
                    }
                    Result::Ok(()) => Result::Ok((converted_amount, conversion_message)),
                }
            }
            ManagedAsyncCallResult::Err(error) => {
                self.checked_decrease_balance(source_token, &payment)?;
                let mut error_msg = ManagedBuffer::new_from_bytes(
                    b"Error when getting the price feed from the aggregator: ",
                );
                error_msg.append(&error.err_msg);

                Result::Err(error_msg)
            }
        }
    }

    #[callback]
    fn finalize_exchange(
        &self,
        #[call_result] result: ManagedAsyncCallResult<OptionalValue<Round<Self::Api>>>,
        caller: ManagedAddress,
        payment: BigUint,
        source_token: TokenIdentifier,
        target_token: TokenIdentifier,
    ) {
        match self.try_convert(result, &payment, &source_token, &target_token) {
            Result::Ok((converted_payment, conversion_message)) => {
                let mut message = ManagedBuffer::new_from_bytes(b"exchange succesful (");
                message.append(&conversion_message);
                message.append_bytes(b")");

                self.send()
                    .direct(&caller, &target_token, 0, &converted_payment, message);
            }
            Result::Err(error) => {
                let mut message = ManagedBuffer::new_from_bytes(b"refund (");
                message.append(&error);
                message.append_bytes(b")");

                self.send()
                    .direct(&caller, &source_token, 0, &payment, message);
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
    ) -> Result<(), ManagedBuffer> {
        match self.balance().get(&token_identifier) {
            Some(balance) => {
                if &balance < amount {
                    let mut err_msg = ManagedBuffer::new_from_bytes(b"Insufficient balance: only ");
                    err_msg.append_bytes(format_biguint(balance).as_slice());
                    err_msg.append_bytes(b" of ");
                    err_msg.append(token_identifier.as_managed_buffer());
                    err_msg.append_bytes(b" available");

                    Result::Err(err_msg)
                } else {
                    self.decrease_balance(token_identifier, amount);
                    Result::Ok(())
                }
            }
            None => {
                let mut err_msg = ManagedBuffer::new_from_bytes(b"No ");
                err_msg.append(token_identifier.as_managed_buffer());
                err_msg.append_bytes(b" tokens are available");

                Result::Err(err_msg)
            }
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
    ) -> Result<ManagedBuffer, ManagedBuffer> {
        let mut message = ManagedBuffer::new_from_bytes(b"conversion from ");
        message.append_bytes(format_biguint(payment.clone()).as_slice());
        message.append_bytes(b" of ");
        message.append(source_token.as_managed_buffer());
        message.append_bytes(b", using exchange rate ");
        message.append_bytes(format_fixed_precision(rate.clone(), rate_precision).as_slice());
        message.append_bytes(b", results in ");
        message.append_bytes(format_biguint(converted_token.clone()).as_slice());
        message.append_bytes(b" of ");
        message.append(target_token.as_managed_buffer());

        Result::Ok(message)
    }

    #[storage_mapper("aggregator")]
    fn aggregator(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("balance")]
    fn balance(&self) -> MapMapper<TokenIdentifier, BigUint>;

    #[proxy]
    fn aggregator_interface_proxy(&self, to: ManagedAddress) -> aggregator::Proxy<Self::Api>;
}
