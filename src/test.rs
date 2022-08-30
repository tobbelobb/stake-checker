#[test]
fn check_with_decimal_point() {
    assert_eq!(with_decimal_point("123"), "0.0000000123");
    assert_eq!(with_decimal_point("12345678905"), "1.2345678905");
    assert_eq!(with_decimal_point("1234567890"), "0.1234567890");
}
