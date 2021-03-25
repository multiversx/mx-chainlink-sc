elrond_wasm::imports!();
elrond_wasm::derive_imports!();
use crate::aggregator_interface::Submission;

/// Calculates the median for each of the values in a Submission
pub fn calculate_submission_median<BigUint: BigUintApi>(
    submissions: Vec<Submission<BigUint>>,
) -> Result<Option<Submission<BigUint>>, SCError> {
    if submissions.is_empty() {
        return Result::Ok(None);
    }
    let values_count = submissions.first().unwrap().values.len();
    let iter = (0..values_count).map(|index| {
        submissions
            .iter()
            .map(|submission| submission.values.iter())
            .flatten()
            .skip(index)
            .step_by(values_count)
    });
    let mut new_submission = Submission::<BigUint> { values: Vec::new() };
    for values in iter {
        let median = calculate(values.cloned().collect())?.unwrap().clone();
        new_submission.values.push(median);
    }
    Result::Ok(Some(new_submission))
}

/// Returns the sorted middle, or the average of the two middle indexed items if the
/// vector has an even number of elements.
pub fn calculate<BigUint: BigUintApi>(mut list: Vec<BigUint>) -> Result<Option<BigUint>, SCError>
where
    BigUint: BigUintApi,
{
    if list.is_empty() {
        return Result::Ok(None);
    }
    list.sort();
    let len = list.len();
    let middle_index = len / 2;
    if len % 2 == 0 {
        let median1 = list.get(middle_index - 1).ok_or("median1 invalid index")?;
        let median2 = list.get(middle_index).ok_or("median2 invalid index")?;
        Result::Ok(Some((median1.clone() + median2.clone()) / 2u64.into()))
    } else {
        let median = list.get(middle_index).ok_or("median invalid index")?;
        Result::Ok(Some(median.clone()))
    }
}
