use elrond_wasm_debug::api::RustBigUint;
use exchange::{format_biguint, format_fixed_precision};

#[test]
fn test_format_biguint() {
    assert_eq!("0", format_biguint(&RustBigUint::from(0u64)));
    assert_eq!(
        "1000000000",
        format_biguint(&RustBigUint::from(1000000000u64))
    );
    assert_eq!(
        "12345678901234567890",
        format_biguint(&RustBigUint::from(12345678901234567890u64))
    );
}

#[test]
fn test_format_fixed_precision() {
    assert_eq!(
        "0.00123",
        format_fixed_precision(&RustBigUint::from(123u64), 5)
    );
    assert_eq!(
        "10000.00000",
        format_fixed_precision(&RustBigUint::from(1000000000u64), 5)
    );
    assert_eq!(
        "123456789012345.67890",
        format_fixed_precision(&RustBigUint::from(12345678901234567890u64), 5)
    );
}
