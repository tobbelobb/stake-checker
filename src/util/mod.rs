#[cfg(test)]
mod util_tests;

use chrono::NaiveDateTime;
use serde::{de, Deserialize, Deserializer};
use serde_json::json;

pub async fn rpc<Params: serde::Serialize>(
    rpc_endpoint: &str,
    method: &str,
    params: Params,
) -> Result<serde_json::Value, reqwest::Error> {
    let client = reqwest::Client::new();
    let resp = client
        .post(rpc_endpoint)
        .json(&json! {{
            "id": 1,
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        }})
        .send()
        .await?;

    let mut ans: serde_json::Value = resp.json().await?;
    Ok(ans["result"].take())
}

pub async fn subquery<Params: serde::Serialize>(
    endpoint: &str,
    query: Params,
) -> Result<serde_json::Value, reqwest::Error> {
    let client = reqwest::Client::new();
    let resp = client
        .post(endpoint)
        .json(&json! {{
            "query": query
        }})
        .send()
        .await?;
    let mut ans: serde_json::Value = resp.json().await?;
    Ok(ans["data"].take())
}

// https://stackoverflow.com/questions/57614558/how-to-use-a-custom-serde-deserializer-for-chrono-timestamps
pub fn naive_date_time_from_str<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error>
where
    D: Deserializer<'de>,
{
    let mut s: String = Deserialize::deserialize(deserializer)?;
    if s.chars().any(|c| c == 'T') {
        const MISSING_SUBSECONDS: &str = "2022-02-03T20:34:00";
        if s.len() == MISSING_SUBSECONDS.len() {
            s.push('.');
        }
        const WELL_FORMED_DATE: &str = "2022-02-03T20:34:00.003";
        while s.len() < WELL_FORMED_DATE.len() {
            s.push('0');
        }
        NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S.%3f").map_err(de::Error::custom)
    } else {
        NaiveDateTime::parse_from_str(&s, "%s").map_err(de::Error::custom)
    }
}

// https://users.rust-lang.org/t/deserialize-a-number-that-may-be-inside-a-string-serde-json/27318
// A custom deserializer, since the value sometimes appear as a quoted string
pub fn balance_from_maybe_str<'de, D>(deserializer: D) -> Result<u128, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    let v = serde_json::Value::deserialize(deserializer)?;
    let n = v
        .as_u64()
        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        .ok_or_else(|| D::Error::custom("non-integer"))?
        .try_into()
        .map_err(|_| D::Error::custom("overflow"))?;
    Ok(n)
}
