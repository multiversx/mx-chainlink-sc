use elrond_wasm_debug::*;

fn world() -> BlockchainMock {
    let mut blockchain = BlockchainMock::new();
    blockchain.register_contract_builder("file:client/output/client.wasm", client::ContractBuilder);
    blockchain.register_contract_builder("file:oracle/output/oracle.wasm", oracle::ContractBuilder);
    blockchain.register_contract_builder(
        "file:aggregator/output/aggregator.wasm",
        aggregator::ContractBuilder,
    );
    blockchain.register_contract_builder(
        "file:price-aggregator/output/price-aggregator.wasm",
        price_aggregator::ContractBuilder,
    );
    blockchain
}

#[test]
fn init() {
    elrond_wasm_debug::mandos_rs("mandos/init.scen.json", world());
}

#[test]
fn client_request() {
    elrond_wasm_debug::mandos_rs("mandos/client-request.scen.json", world());
}

#[test]
fn aggregator() {
    elrond_wasm_debug::mandos_rs("mandos/aggregator.scen.json", world());
}

#[test]
fn init_price_aggregator() {
    elrond_wasm_debug::mandos_rs("mandos/init-price-aggregator.scen.json", world());
}

#[test]
fn price_aggregator() {
    elrond_wasm_debug::mandos_rs("mandos/price-aggregator.scen.json", world());
}

#[test]
fn price_aggregator_balance() {
    elrond_wasm_debug::mandos_rs("mandos/price-aggregator-balance.scen.json", world());
}
