use aggregator::aggregator_interface::{Submission, MAX_SUBMISSION_VALUES};
use aggregator::median;
use elrond_wasm::arrayvec::ArrayVec;
use elrond_wasm::types::BigUint;
use elrond_wasm_debug::DebugApi;

fn to_vec_biguint(v: Vec<u32>) -> ArrayVec<BigUint<DebugApi>, MAX_SUBMISSION_VALUES> {
    v.iter()
        .map(|value| BigUint::<DebugApi>::from(*value))
        .collect()
}

fn check_median_result(expected: Option<u32>, v: Vec<u32>) {
    let expected_biguint: Option<BigUint<DebugApi>> =
        expected.map(|value| BigUint::<DebugApi>::from(value));
    let actual_result = median::calculate::<DebugApi>(to_vec_biguint(v));
    assert_eq!(Result::Ok(expected_biguint), actual_result);
}

#[test]
fn test_median() {
    let _ = DebugApi::dummy();

    // empty list
    check_median_result(None, vec![]);

    // odd number of elements
    check_median_result(Some(42), vec![42]);
    check_median_result(Some(11), vec![10, 11, 12]);
    check_median_result(Some(15), vec![20, 10, 15, 17, 19, 11, 12]);
    check_median_result(Some(10), vec![10, 9, 8, 7, 6, 5, 11, 12, 13, 14, 15]);
    check_median_result(Some(10), vec![9, 8, 7, 6, 5, 11, 12, 13, 14, 10, 15]);

    // even number of elements
    check_median_result(Some(42), vec![42, 43]);
    check_median_result(Some(11), vec![10, 11, 12, 13]);
    check_median_result(Some(15), vec![20, 10, 15, 16, 17, 19, 11, 12]);
    check_median_result(Some(10), vec![10, 9, 8, 7, 6, 5, 11, 12, 13, 14, 15, 16]);
    check_median_result(Some(10), vec![9, 8, 7, 6, 5, 11, 12, 13, 14, 10, 15, 16]);
}

#[test]
fn test_median_equal() {
    let _ = DebugApi::dummy();
    check_median_result(Some(42), vec![42]);
    check_median_result(Some(42), vec![42, 42]);
    check_median_result(Some(42), vec![42, 42, 42]);
    check_median_result(Some(42), vec![42, 42, 42, 42]);
}

#[test]
fn test_median_submission_empty() {
    let actual_result = median::calculate_submission_median::<DebugApi>(ArrayVec::new()).unwrap();
    assert!(actual_result.is_none());
}

#[test]
fn test_median_submission() {
    let _ = DebugApi::dummy();
    let submission_a = Submission {
        values: to_vec_biguint(vec![100, 5000, 6000, 7000, 200, 300, 400]),
    };
    let submission_b = Submission {
        values: to_vec_biguint(vec![110, 5010, 6010, 7010, 210, 310, 410]),
    };
    let expected_submission_result = Submission {
        values: to_vec_biguint(vec![105, 5005, 6005, 7005, 205, 305, 405]),
    };

    let mut a_b_as_vec = ArrayVec::new();
    a_b_as_vec.push(submission_a);
    a_b_as_vec.push(submission_b);

    let actual_result = median::calculate_submission_median(a_b_as_vec)
        .unwrap()
        .unwrap();
    assert_eq!(actual_result.values, expected_submission_result.values);
}
