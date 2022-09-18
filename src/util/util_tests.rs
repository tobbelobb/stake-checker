use crate::util::naive_date_time_from_str;
use serde::de::value::StrDeserializer;

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
