use crate::*;
use chrono::NaiveDate;

use std::io::Write;

use mockito::mock;

#[test]
fn check_with_decimal_point_strings() {
    assert_eq!("123".with_decimal_point(10), "0.0000000123");
    assert_eq!("12345678905".with_decimal_point(10), "1.2345678905");
    assert_eq!("1234567890".with_decimal_point(10), "0.1234567890");
}

#[test]
fn check_with_decimal_point_u128() {
    // As long as u128 is the only candidate type to implement this trait,
    // Rust will actually take a guess for us here...
    //assert_eq!(123u128.with_decimal_point(), "0.0000000123");
    assert_eq!(123.with_decimal_point(10), "0.0000000123");
    assert_eq!(12345678905.with_decimal_point(10), "1.2345678905");
    assert_eq!(1234567890.with_decimal_point(10), "0.1234567890");
}

#[test]
fn read_known_stake_changes() -> Result<(), Box<dyn std::error::Error>> {
    let _known_stake_changes = known_stake_changes("./src/known_stake_changes_test.csv")?;
    Ok(())
}

#[tokio::test]
async fn get_total_issuance_happy_case() -> Result<(), Box<dyn std::error::Error>> {
    let mock = mock("POST", "/")
        .with_status(200)
        .with_header("content-type", "application/json;charset=utf-8")
        .with_body(
            "{ \"id\": 1, \"jsonrpc\": \"2.0\", \"result\": \"0x8a90a53a59f376aa0000000000000000\"}",
        )
        .create();
    let rpc_endpoint = mockito::server_url();

    let total_issuance = get_total_issuance(&rpc_endpoint).await?;

    mock.assert();
    assert_eq!(total_issuance, 12283272598261174410);
    Ok(())
}

#[tokio::test]
async fn get_stake_changes_happy_case() -> Result<(), Box<dyn std::error::Error>> {
    // Simulate a subquery server that says three rewards exist
    let mock = mock("POST", "/")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            "{\"data\":\
                {\"stakeChanges\":\
                    {\"nodes\":\
                        [\
                            {\"accumulatedAmount\":\"1000000000000\",\"timestamp\":\"1663610000\"},\
                            {\"accumulatedAmount\":\"2000000000000\",\"timestamp\":\"1663620000\"},\
                            {\"accumulatedAmount\":\"3000000000000\",\"timestamp\":\"1663630000\"}\
                        ]\
                    }\
                 }\
             }",
        )
        .create();
    let subquery_endpoint = mockito::server_url();

    // Simulate two known data points
    let dummy_file_name = testfile::generate_name();
    let mut f = std::fs::File::create(&dummy_file_name).unwrap();
    f.write(
        "2022-09-19T17:53:20.000,1000000000000\n2022-09-19T20:40:00.000,2000000000000\n".as_bytes(),
    )
    .expect("Failed to write to tmp file");
    let _tf = testfile::from_file(&dummy_file_name); // Takes care of deleting tmp file

    let found_stake_changes = get_stake_changes(
        &subquery_endpoint,
        "dummyAddress",
        dummy_file_name.to_str().unwrap(),
    )
    .await?;

    mock.assert();
    assert_eq!(found_stake_changes.len(), 1);
    assert_eq!(
        found_stake_changes[0],
        StakeChange {
            timestamp: NaiveDate::from_ymd(2022, 9, 19).and_hms(23, 26, 40),
            accumulated_amount: 3000000000000
        }
    );
    Ok(())
}

#[tokio::test]
async fn get_staking_rewards_happy_case() -> Result<(), Box<dyn std::error::Error>> {
    // Simulate a subquery server that says three rewards exist
    let mock = mock("POST", "/")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            "{\"data\":\
                {\"stakingRewards\":\
                    {\"nodes\":\
                        [\
                            {\"balance\":\"9\",\"date\":\"2015-06-10T08:07:06.011\"},\
                            {\"balance\":\"10\",\"date\":\"2015-06-11T08:07:06.011\"},\
                            {\"balance\":\"11\",\"date\":\"2016-07-08T09:10:11.000\"}\
                        ]\
                    }\
                 }\
             }",
        )
        .create();
    let subquery_endpoint = mockito::server_url();

    // Simulate two known data points
    let dummy_file_name = testfile::generate_name();
    let mut f = std::fs::File::create(&dummy_file_name).unwrap();
    f.write("2015-06-10T08:07:06.011,10\n2015-06-11T08:07:06.011,10\n".as_bytes())
        .expect("Failed to write to tmp file");
    let _tf = testfile::from_file(&dummy_file_name);

    let found_rewards = get_staking_rewards(
        &subquery_endpoint,
        "dummyAddress",
        dummy_file_name.to_str().unwrap(),
    )
    .await?;

    mock.assert();
    assert_eq!(found_rewards.len(), 1);
    assert_eq!(
        found_rewards[0],
        Reward {
            date: NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11),
            balance: 11
        }
    );
    Ok(())
}
