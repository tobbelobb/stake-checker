use crate::{
    get_staking_rewards, get_total_issuance, naive_date_time_from_str, DecimalPointPuttable, Reward,
};
use chrono::NaiveDate;

use mockito::mock;
use serde::de::value::StrDeserializer;
use std::io::Write;
//use testfile;

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
    let _ignored = f.write("2015-06-10T08:07:06.011,10\n2015-06-11T08:07:06.011,10\n".as_bytes());
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

#[test]
fn read_date_simplest_case() -> Result<(), Box<dyn std::error::Error>> {
    let date_str = "2022-02-03T20:34:00.003";
    let date_time = naive_date_time_from_str(StrDeserializer::<serde_json::Error>::new(date_str))?;
    let back_to_str = format!("{:?}", date_time);
    assert_eq!(back_to_str, "2022-02-03T20:34:00.003");
    Ok(())
}

#[test]
fn read_date_missing_millisecond() -> Result<(), Box<dyn std::error::Error>> {
    let date_str = "2022-04-01T18:27:12.01";
    let date_time = naive_date_time_from_str(StrDeserializer::<serde_json::Error>::new(date_str))?;
    let back_to_str = format!("{:?}", date_time);
    assert_eq!(back_to_str, "2022-04-01T18:27:12.010");
    Ok(())
}
