#![no_std]

elrond_wasm::imports!();

pub const EGLD_TICKER: &[u8] = b"EGLD";
pub const DOLLAR_TICKER: &[u8] = b"USD";

pub type AggregatorResultAsMultiResult<M> =
    MultiResult5<u32, ManagedBuffer<M>, ManagedBuffer<M>, BigUint<M>, u8>;

mod price_aggregator_proxy {
    elrond_wasm::imports!();

    #[elrond_wasm::proxy]
    pub trait PriceAggregator {
        #[view(latestPriceFeedOptional)]
        fn latest_price_feed_optional(
            &self,
            from: ManagedBuffer,
            to: ManagedBuffer,
        ) -> OptionalResult<super::AggregatorResultAsMultiResult<Self::Api>>;
    }
}

pub struct AggregatorResult<M: ManagedTypeApi> {
    pub round_id: u32,
    pub from_token_name: ManagedBuffer<M>,
    pub to_token_name: ManagedBuffer<M>,
    pub price: BigUint<M>,
    pub decimals: u8,
}

impl<M: ManagedTypeApi> From<AggregatorResultAsMultiResult<M>>
    for AggregatorResult<M>
{
    fn from(multi_result: AggregatorResultAsMultiResult<M>) -> Self {
        let (round_id, from_token_name, to_token_name, price, decimals) = multi_result.into_tuple();

        AggregatorResult {
            round_id,
            from_token_name,
            to_token_name,
            price,
            decimals,
        }
    }
}

#[elrond_wasm::module]
pub trait PriceAggregatorModule {
    #[only_owner]
    #[endpoint(setPriceAggregatorAddress)]
    fn set_price_aggregator_address(&self, address: ManagedAddress) {
        require!(
            self.blockchain().is_smart_contract(&address),
            "Invalid price aggregator address"
        );

        self.price_aggregator_address().set(&address);
    }

    fn get_price_for_pair(
        &self,
        from_ticker: ManagedBuffer,
        to_ticker: ManagedBuffer,
    ) -> Option<BigUint> {
        self.get_full_result_for_pair(from_ticker, to_ticker)
            .map(|aggregator_result| aggregator_result.price)
    }

    fn get_full_result_for_pair(
        &self,
        from_ticker: ManagedBuffer,
        to_ticker: ManagedBuffer,
    ) -> Option<AggregatorResult<Self::Api>> {
        let price_aggregator_address = self.price_aggregator_address().get();
        if price_aggregator_address.is_zero() {
            return None;
        }

        let result: OptionalResult<AggregatorResultAsMultiResult<Self::Api>> = self
            .aggregator_proxy(price_aggregator_address)
            .latest_price_feed_optional(from_ticker, to_ticker)
            .execute_on_dest_context();

        result
            .into_option()
            .map(|multi_result| AggregatorResult::from(multi_result))
    }

    #[proxy]
    fn aggregator_proxy(&self, address: ManagedAddress) -> price_aggregator_proxy::Proxy<Self::Api>;

    #[view(getAggregatorAddress)]
    #[storage_mapper("priceAggregatorAddress")]
    fn price_aggregator_address(&self) -> SingleValueMapper<ManagedAddress>;
}
