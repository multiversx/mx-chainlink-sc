use elrond_wasm::types::BigUint;
use elrond_wasm_debug::DebugApi;
use exchange::{format_biguint, format_fixed_precision};

#[test]
fn test_format_biguint() {
    let _ = DebugApi::dummy();
    assert_eq!("0".as_bytes(), format_biguint(&BigUint::<DebugApi>::zero()));
    assert_eq!(
        "1000000000".to_string(),
        String::from_utf8(format_biguint(&BigUint::<DebugApi>::from(1000000000u64))).unwrap()
    );
    assert_eq!(
        "1234567890".to_string(),
        String::from_utf8(format_biguint(&BigUint::<DebugApi>::from(1234567890u64))).unwrap()
    );
}

// TODO: fix & enable
// #[test]
// fn test_format_fixed_precision() {
//     let api = DebugApi::dummy();
//     assert_eq!(
//         "0.00123".to_string(),
//         String::from_utf8(format_fixed_precision(&123u64.managed_into(api.clone()), 5)).unwrap()
//     );
//     assert_eq!(
//         "10000.00000".to_string(),
//         String::from_utf8(format_fixed_precision(
//             &1000000000u64.managed_into(api.clone()),
//             5
//         ))
//         .unwrap()
//     );
//     assert_eq!(
//         "123456789012345.67890".to_string(),
//         String::from_utf8(format_fixed_precision(
//             &12345678901234567890u64.managed_into(api.clone()),
//             5
//         ))
//         .unwrap()
//     );
// }
