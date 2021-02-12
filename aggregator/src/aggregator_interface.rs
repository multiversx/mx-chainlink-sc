imports!();
derive_imports!();

#[derive(NestedEncode, NestedDecode, TopEncode, TopDecode, TypeAbi)]
pub struct RoundData {
    pub round_id: u64,
    pub answer: u64,
    pub started_at: u64,
    pub updated_at: u64,
    pub answered_in_round: u64,
}

pub trait AggregatorInterface {
    fn decimals(&self) -> u8;
    fn description(&self) -> BoxedBytes;
    fn version(&self) -> u64;

    fn get_round_data(&self, round_id: u64) -> SCResult<RoundData>;
    fn latest_round_data(&self) -> SCResult<RoundData>;
}
