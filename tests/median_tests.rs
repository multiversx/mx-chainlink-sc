use aggregator::median;
use elrond_wasm_debug::api::RustBigUint;

fn to_vec_biguint(v: Vec<u32>) -> Vec<RustBigUint> {
    v.iter().map(|value| (*value as u64).into()).collect()
}

fn check_median_result(expected: u32, v: Vec<u32>) {
    let expected_biguint: RustBigUint = expected.into();
    let actual_result = median::calculate::<RustBigUint>(&to_vec_biguint(v));
    assert_eq!(expected_biguint, actual_result);
}

#[test]
fn test_median() {
    // odd number of elements
    check_median_result(42, vec![42]);
    check_median_result(11, vec![10, 11, 12]);
    check_median_result(15, vec![20, 10, 15, 17, 19, 11, 12]);
    check_median_result(10, vec![10, 9, 8, 7, 6, 5, 11, 12, 13, 14, 15]);
    check_median_result(10, vec![9, 8, 7, 6, 5, 11, 12, 13, 14, 10, 15]);

    // even number of elements
    check_median_result(42, vec![42, 43]);
    check_median_result(11, vec![10, 11, 12, 13]);
    check_median_result(15, vec![20, 10, 15, 16, 17, 19, 11, 12]);
    check_median_result(10, vec![10, 9, 8, 7, 6, 5, 11, 12, 13, 14, 15, 16]);
    check_median_result(10, vec![9, 8, 7, 6, 5, 11, 12, 13, 14, 10, 15, 16]);
}
