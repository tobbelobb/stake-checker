use crate::{get_total_issuance, DecimalPointPuttable};
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
