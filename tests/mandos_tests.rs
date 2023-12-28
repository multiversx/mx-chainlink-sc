use multiversx_sc_scenario::*;

fn world() -> ScenarioWorld {
    let mut blockchain = ScenarioWorld::new();
    blockchain.register_contract("file:client/output/client.wasm", client::ContractBuilder);
    blockchain.register_contract("file:oracle/output/oracle.wasm", oracle::ContractBuilder);
    blockchain.register_contract(
        "file:aggregator/output/aggregator.wasm",
        aggregator::ContractBuilder,
    );
    blockchain.register_contract(
        "file:price-aggregator/output/price-aggregator.wasm",
        price_aggregator::ContractBuilder,
    );
    blockchain
}

#[test]
fn init() {
    multiversx_sc_scenario::run_rs("mandos/init.scen.json", world());
}

#[test]
fn client_request() {
    multiversx_sc_scenario::run_rs("mandos/client-request.scen.json", world());
}

#[test]
fn aggregator() {
    multiversx_sc_scenario::run_rs("mandos/aggregator.scen.json", world());
}
