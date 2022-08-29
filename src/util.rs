use serde_json::{json, Value};

pub async fn rpc<Params: serde::Serialize>(
    rpc_endpoint: &str,
    method: &str,
    _params: Params,
) -> Result<Value, reqwest::Error> {
    let client = reqwest::Client::new();
    let resp = client
        .post(rpc_endpoint)
        .json(&json! {{
            "id": 1,
            "jsonrpc": "2.0",
            "method": method,
        }})
        .send()
        .await?;

    let mut ans: Value = resp.json().await?;
    Ok(ans["result"].take())
}
