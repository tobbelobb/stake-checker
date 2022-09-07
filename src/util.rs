use serde_json::{json, Value};

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

    let mut ans: Value = resp.json().await?;
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
    let mut ans: Value = resp.json().await?;
    Ok(ans["data"].take())
}
