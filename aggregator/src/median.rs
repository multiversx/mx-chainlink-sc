elrond_wasm::imports!();
elrond_wasm::derive_imports!();

/**
* @notice Returns the sorted middle, or the average of the two middle indexed items if the
* array has an even number of elements.
* @dev The list passed as an argument isn't modified.
* @dev This algorithm has expected runtime O(n), but for adversarially chosen inputs
* the runtime is O(n^2).
* @param list The list of elements to compare
*/
pub fn calculate<BigUint: BigUintApi>(list: &Vec<BigUint>) -> BigUint {
    calculate_inplace(list.clone())
}

/**
* @notice See documentation for function calculate.
* @dev The list passed as an argument may be permuted.
*/
fn calculate_inplace<BigUint>(mut list: Vec<BigUint>) -> BigUint
where
    BigUint: BigUintApi,
{
    assert!(!list.is_empty(), "list must not be empty");
    let len = list.len();
    let middle_index = len / 2;
    if len % 2 == 0 {
        let (median1, median2) =
            quickselect_two(&mut list, 0, len - 1, middle_index - 1, middle_index);
        return (median1 + median2) / 2u64.into();
    } else {
        return quickselect(&mut list, 0, len - 1, middle_index);
    }
}

/**
* @notice Selects the k-th ranked element from list, looking only at indices between lo and hi
* (inclusive). Modifies list in-place.
*/
fn quickselect<BigUint: BigUintApi>(
    list: &mut Vec<BigUint>,
    mut lo: usize,
    mut hi: usize,
    k: usize,
) -> BigUint {
    assert!(lo <= k);
    assert!(k <= hi);
    while lo < hi {
        let pivot_index = partition(list, lo, hi);
        if k <= pivot_index {
            // since pivotIndex < (original hi passed to partition),
            // termination is guaranteed in this case
            hi = pivot_index;
        } else {
            // since (original lo passed to partition) <= pivotIndex,
            // termination is guaranteed in this case
            lo = pivot_index + 1;
        }
    }
    return list[lo].clone();
}

/**
* @notice Selects the k1-th and k2-th ranked elements from list, looking only at indices between
* lo and hi (inclusive). Modifies list in-place.
*/
fn quickselect_two<BigUint: BigUintApi>(
    list: &mut Vec<BigUint>,
    mut lo: usize,
    mut hi: usize,
    k1: usize,
    k2: usize,
) -> (BigUint, BigUint) {
    assert!(k1 < k2);
    assert!(lo <= k1 && k1 <= hi);
    assert!(lo <= k2 && k2 <= hi);

    loop {
        let pivot_idx = partition(list, lo, hi);
        if k2 <= pivot_idx {
            hi = pivot_idx;
        } else if pivot_idx < k1 {
            lo = pivot_idx + 1;
        } else {
            assert!(k1 <= pivot_idx && pivot_idx < k2);
            let k1th = quickselect(list, lo, pivot_idx, k1);
            let k2th = quickselect(list, pivot_idx + 1, hi, k2);
            return (k1th, k2th);
        }
    }
}

/**
* @notice Partitions list in-place using Hoare's partitioning scheme.
* Only elements of list between indices lo and hi (inclusive) will be modified.
* Returns an index i, such that:
* - lo <= i < hi
* - forall j in [lo, i]. list[j] <= list[i]
* - forall j in [i, hi]. list[i] <= list[j]
*/
fn partition<BigUint: BigUintApi>(list: &mut Vec<BigUint>, mut lo: usize, mut hi: usize) -> usize {
    // We don't care about overflow of the addition, because it would require a list
    // larger than any feasible computer's memory.
    let pivot = list[(lo + hi) / 2].clone();
    lo = lo.overflowing_sub(1).0; // this can underflow. that's intentional.
    hi += 1;
    loop {
        loop {
            lo = lo.overflowing_add(1).0;
            if !(list[lo] < pivot) {
                break;
            }
        }
        loop {
            hi -= 1;
            if !(list[hi] > pivot) {
                break;
            }
        }
        if lo < hi {
            (list[lo], list[hi]) = (list[hi].clone(), list[lo].clone());
        } else {
            // Let orig_lo and orig_hi be the original values of lo and hi passed to partition.
            // Then, hi < orig_hi, because hi decreases *strictly* monotonically
            // in each loop iteration and
            // - either list[orig_hi] > pivot, in which case the first loop iteration
            //   will achieve hi < orig_hi;
            // - or list[orig_hi] <= pivot, in which case at least two loop iterations are
            //   needed:
            //   - lo will have to stop at least once in the interval
            //     [orig_lo, (orig_lo + orig_hi)/2]
            //   - (orig_lo + orig_hi)/2 < orig_hi
            return hi;
        }
    }
}
