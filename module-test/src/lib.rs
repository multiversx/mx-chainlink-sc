#![no_std]

multiversx_sc::imports!();

use price_aggregator_proxy::{DOLLAR_TICKER, EGLD_TICKER};

#[multiversx_sc::contract]
pub trait ModuleTest: price_aggregator_proxy::PriceAggregatorModule {
    #[init]
    fn init(&self, price_aggregator_address: ManagedAddress) {
        self.price_aggregator_address()
            .set(&price_aggregator_address);
    }

    fn call_price_aggregatpr(&self) -> BigUint {
        self.get_price_for_pair(
            ManagedBuffer::from(EGLD_TICKER),
            ManagedBuffer::from(DOLLAR_TICKER),
        )
        .unwrap_or_else(|| BigUint::zero())
    }
}
