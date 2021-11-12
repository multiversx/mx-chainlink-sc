use elrond_wasm::*;
use elrond_wasm_debug::*;

fn world() -> BlockchainMock {
    let mut blockchain = BlockchainMock::new();
    blockchain.register_contract(
        "file:client/output/client.wasm",
        Box::new(|context| Box::new(client::contract_obj(context))),
    );
    blockchain.register_contract(
        "file:oracle/output/oracle.wasm",
        Box::new(|context| Box::new(oracle::contract_obj(context))),
    );
    blockchain.register_contract(
        "file:aggregator/output/aggregator.wasm",
        Box::new(|context| Box::new(aggregator::contract_obj(context))),
    );
    blockchain.register_contract(
        "file:price-aggregator/output/price-aggregator.wasm",
        Box::new(|context| Box::new(price_aggregator::contract_obj(context))),
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
