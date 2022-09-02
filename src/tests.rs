use crate::DecimalPointPuttable;

#[test]
fn check_with_decimal_point_strings() {
    assert_eq!("123".with_decimal_point(), "0.0000000123");
    assert_eq!("12345678905".with_decimal_point(), "1.2345678905");
    assert_eq!("1234567890".with_decimal_point(), "0.1234567890");
}

#[test]
fn check_with_decimal_point_u128() {
    // As long as u128 is the only candidate type to implement this trait,
    // Rust will actually take a guess for us here...
    //assert_eq!(123u128.with_decimal_point(), "0.0000000123");
    assert_eq!(123.with_decimal_point(), "0.0000000123");
    assert_eq!(12345678905.with_decimal_point(), "1.2345678905");
    assert_eq!(1234567890.with_decimal_point(), "0.1234567890");
}
