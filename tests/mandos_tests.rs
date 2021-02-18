use aggregator::AggregatorImpl;
use client::ClientImpl;
use oracle::OracleImpl;

use elrond_wasm::*;
use elrond_wasm_debug::*;

fn contract_map() -> ContractMap<TxContext> {
    let mut contract_map = ContractMap::new();
    contract_map.register_contract(
        "file:../client/output/client.wasm",
        Box::new(|context| Box::new(ClientImpl::new(context))),
    );
    contract_map.register_contract(
        "file:../oracle/output/oracle.wasm",
        Box::new(|context| Box::new(OracleImpl::new(context))),
    );
    contract_map.register_contract(
        "file:../aggregator/output/aggregator.wasm",
        Box::new(|context| Box::new(AggregatorImpl::new(context))),
    );
    contract_map
}

#[test]
fn init() {
    parse_execute_mandos("mandos/init.scen.json", &contract_map());
}

#[test]
fn client_request() {
    parse_execute_mandos("mandos/client-request.scen.json", &contract_map());
}

#[test]
fn aggregator() {
    parse_execute_mandos("mandos/aggregator.scen.json", &contract_map());
}
