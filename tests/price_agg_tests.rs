use elrond_wasm::types::{Address, MultiValueEncoded};
use elrond_wasm_debug::{
    managed_address, managed_biguint, managed_buffer, rust_biguint,
    testing_framework::BlockchainStateWrapper,
};
use elrond_wasm_modules::pause::PauseModule;
use price_aggregator::{
    price_aggregator_data::{OracleStatus, TimestampedPrice, TokenPair},
    PriceAggregator, MAX_ROUND_DURATION_SECONDS,
};

const SUBMISSION_COUNT: usize = 3;
const DECIMALS: u8 = 0;
static EGLD_TICKER: &[u8] = b"EGLD";
static USD_TICKER: &[u8] = b"USDC";

#[test]
fn price_agg_submit_test() {
    let rust_zero = rust_biguint!(0);
    let mut b_mock = BlockchainStateWrapper::new();
    let owner = b_mock.create_user_account(&rust_zero);

    let mut oracles = [
        Address::zero(),
        Address::zero(),
        Address::zero(),
        Address::zero(),
    ];
    for i in 0..4 {
        oracles[i] = b_mock.create_user_account(&rust_zero);
    }

    let price_agg = b_mock.create_sc_account(
        &rust_zero,
        Some(&owner),
        price_aggregator::contract_obj,
        "price_agg_path",
    );

    let current_timestamp = 100;
    b_mock.set_block_timestamp(current_timestamp);

    // init price aggregator
    b_mock
        .execute_tx(&owner, &price_agg, &rust_zero, |sc| {
            let mut oracle_args = MultiValueEncoded::new();
            for oracle in &oracles {
                oracle_args.push(managed_address!(oracle));
            }

            sc.init(SUBMISSION_COUNT, DECIMALS, oracle_args);
        })
        .assert_ok();

    // try submit while paused
    b_mock
        .execute_tx(&oracles[0], &price_agg, &rust_zero, |sc| {
            sc.submit(
                managed_buffer!(EGLD_TICKER),
                managed_buffer!(USD_TICKER),
                99,
                managed_biguint!(100),
            );
        })
        .assert_user_error("Contract is paused");

    // unpause
    b_mock
        .execute_tx(&owner, &price_agg, &rust_zero, |sc| {
            sc.unpause_endpoint();
        })
        .assert_ok();

    // submit first timestamp too old
    b_mock
        .execute_tx(&oracles[0], &price_agg, &rust_zero, |sc| {
            sc.submit(
                managed_buffer!(EGLD_TICKER),
                managed_buffer!(USD_TICKER),
                10,
                managed_biguint!(100),
            );
        })
        .assert_user_error("First submission too old");

    // submit ok
    b_mock
        .execute_tx(&oracles[0], &price_agg, &rust_zero, |sc| {
            sc.submit(
                managed_buffer!(EGLD_TICKER),
                managed_buffer!(USD_TICKER),
                95,
                managed_biguint!(100),
            );

            let token_pair = TokenPair {
                from: managed_buffer!(EGLD_TICKER),
                to: managed_buffer!(USD_TICKER),
            };
            assert_eq!(
                sc.first_submission_timestamp(&token_pair).get(),
                current_timestamp
            );
            assert_eq!(
                sc.last_submission_timestamp(&token_pair).get(),
                current_timestamp
            );

            let submissions = sc.submissions().get(&token_pair).unwrap();
            assert_eq!(submissions.len(), 1);
            assert_eq!(
                submissions.get(&managed_address!(&oracles[0])).unwrap(),
                managed_biguint!(100)
            );

            assert_eq!(
                sc.oracle_status()
                    .get(&managed_address!(&oracles[0]))
                    .unwrap(),
                OracleStatus {
                    total_submissions: 1,
                    accepted_submissions: 1
                }
            );
        })
        .assert_ok();

    // first oracle submit again - submission not accepted
    b_mock
        .execute_tx(&oracles[0], &price_agg, &rust_zero, |sc| {
            sc.submit(
                managed_buffer!(EGLD_TICKER),
                managed_buffer!(USD_TICKER),
                95,
                managed_biguint!(100),
            );

            assert_eq!(
                sc.oracle_status()
                    .get(&managed_address!(&oracles[0]))
                    .unwrap(),
                OracleStatus {
                    total_submissions: 2,
                    accepted_submissions: 1
                }
            );
        })
        .assert_ok();
}

#[test]
fn price_agg_submit_round_ok_test() {
    let rust_zero = rust_biguint!(0);
    let mut b_mock = BlockchainStateWrapper::new();
    let owner = b_mock.create_user_account(&rust_zero);

    let mut oracles = [
        Address::zero(),
        Address::zero(),
        Address::zero(),
        Address::zero(),
    ];
    for i in 0..4 {
        oracles[i] = b_mock.create_user_account(&rust_zero);
    }

    let price_agg = b_mock.create_sc_account(
        &rust_zero,
        Some(&owner),
        price_aggregator::contract_obj,
        "price_agg_path",
    );

    let mut current_timestamp = 100;
    b_mock.set_block_timestamp(current_timestamp);

    // init price aggregator
    b_mock
        .execute_tx(&owner, &price_agg, &rust_zero, |sc| {
            let mut oracle_args = MultiValueEncoded::new();
            for oracle in &oracles {
                oracle_args.push(managed_address!(oracle));
            }

            sc.init(SUBMISSION_COUNT, DECIMALS, oracle_args);
            sc.unpause_endpoint();
        })
        .assert_ok();

    // submit first
    b_mock
        .execute_tx(&oracles[0], &price_agg, &rust_zero, |sc| {
            sc.submit(
                managed_buffer!(EGLD_TICKER),
                managed_buffer!(USD_TICKER),
                95,
                managed_biguint!(10_000),
            );
        })
        .assert_ok();

    current_timestamp = 110;
    b_mock.set_block_timestamp(current_timestamp);

    // submit second
    b_mock
        .execute_tx(&oracles[1], &price_agg, &rust_zero, |sc| {
            sc.submit(
                managed_buffer!(EGLD_TICKER),
                managed_buffer!(USD_TICKER),
                101,
                managed_biguint!(11_000),
            );
        })
        .assert_ok();

    // submit third
    b_mock
        .execute_tx(&oracles[2], &price_agg, &rust_zero, |sc| {
            sc.submit(
                managed_buffer!(EGLD_TICKER),
                managed_buffer!(USD_TICKER),
                105,
                managed_biguint!(12_000),
            );
        })
        .assert_ok();

    b_mock
        .execute_query(&price_agg, |sc| {
            let result = sc
                .latest_price_feed(managed_buffer!(EGLD_TICKER), managed_buffer!(USD_TICKER))
                .unwrap();

            let (round_id, from, to, timestamp, price, decimals) = result.into_tuple();
            assert_eq!(round_id, 1);
            assert_eq!(from, managed_buffer!(EGLD_TICKER));
            assert_eq!(to, managed_buffer!(USD_TICKER));
            assert_eq!(timestamp, current_timestamp);
            assert_eq!(price, managed_biguint!(11_000));
            assert_eq!(decimals, DECIMALS);

            // submissions are deleted after round is created
            let token_pair = TokenPair { from, to };
            let submissions = sc.submissions().get(&token_pair).unwrap();
            assert_eq!(submissions.len(), 0);

            let rounds = sc.rounds().get(&token_pair).unwrap();
            assert_eq!(rounds.len(), 1);
            assert_eq!(rounds.get(1), TimestampedPrice { timestamp, price });
        })
        .assert_ok();
}

#[test]
fn price_agg_discarded_round_test() {
    let rust_zero = rust_biguint!(0);
    let mut b_mock = BlockchainStateWrapper::new();
    let owner = b_mock.create_user_account(&rust_zero);

    let mut oracles = [
        Address::zero(),
        Address::zero(),
        Address::zero(),
        Address::zero(),
    ];
    for i in 0..4 {
        oracles[i] = b_mock.create_user_account(&rust_zero);
    }

    let price_agg = b_mock.create_sc_account(
        &rust_zero,
        Some(&owner),
        price_aggregator::contract_obj,
        "price_agg_path",
    );

    let mut current_timestamp = 100;
    b_mock.set_block_timestamp(current_timestamp);

    // init price aggregator
    b_mock
        .execute_tx(&owner, &price_agg, &rust_zero, |sc| {
            let mut oracle_args = MultiValueEncoded::new();
            for oracle in &oracles {
                oracle_args.push(managed_address!(oracle));
            }

            sc.init(SUBMISSION_COUNT, DECIMALS, oracle_args);
            sc.unpause_endpoint();
        })
        .assert_ok();

    // submit first
    b_mock
        .execute_tx(&oracles[0], &price_agg, &rust_zero, |sc| {
            sc.submit(
                managed_buffer!(EGLD_TICKER),
                managed_buffer!(USD_TICKER),
                95,
                managed_biguint!(10_000),
            );
        })
        .assert_ok();

    current_timestamp += MAX_ROUND_DURATION_SECONDS + 1;
    b_mock.set_block_timestamp(current_timestamp);

    // submit second - this will discard the previous submission
    b_mock
        .execute_tx(&oracles[1], &price_agg, &rust_zero, |sc| {
            sc.submit(
                managed_buffer!(EGLD_TICKER),
                managed_buffer!(USD_TICKER),
                current_timestamp - 1,
                managed_biguint!(11_000),
            );
        })
        .assert_ok();

    b_mock
        .execute_query(&price_agg, |sc| {
            let token_pair = TokenPair {
                from: managed_buffer!(EGLD_TICKER),
                to: managed_buffer!(USD_TICKER),
            };
            let submissions = sc.submissions().get(&token_pair).unwrap();
            assert_eq!(submissions.len(), 1);
            assert_eq!(
                submissions.get(&managed_address!(&oracles[1])).unwrap(),
                managed_biguint!(11_000)
            );
        })
        .assert_ok();
}
