extern crate client;
use client::*;

extern crate oracle;
use oracle::*;

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
    contract_map
}

#[test]
fn client_request() {
    parse_execute_mandos("mandos/client-request.scen.json", &contract_map());
}
