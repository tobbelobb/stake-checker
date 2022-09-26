#[cfg(test)]
mod cli_tests;
#[cfg(test)]
mod tests;

mod util;

use std::fmt;
use std::fs;
use std::path::Path;

use chrono::NaiveDateTime;
use csv::ReaderBuilder;
use frame_metadata::RuntimeMetadataPrefixed;
use parity_scale_codec::Decode;
use serde::Deserialize;
use sp_core::{crypto::AccountId32, crypto::PublicError, crypto::Ss58Codec, hashing};
pub type TokenDecimals = usize;

pub type PolkadotAccountInfo = pallet_system::AccountInfo<u32, pallet_balances::AccountData<u128>>;
pub type Stringifier = fn(&[u8], TokenDecimals) -> Result<String, ScError>;

pub enum ScError {
    NoEnvFile,
    NoRpcEndpoint,
    NoSubqueryEndpoint,
    NoPolkadotAddr,
    InvalidPolkadotAddr,
    NoDataFound,
    IO(std::io::Error),
    Reqwest(reqwest::Error),
    Crypto(PublicError),
    Codec(parity_scale_codec::Error),
    Json(serde_json::Error),
    Csv(csv::Error),
}

impl std::error::Error for ScError {}

impl fmt::Display for ScError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ScError::NoRpcEndpoint => {
                write!(f, "No RPC_ENDPOINT set in .env")
            }
            ScError::NoSubqueryEndpoint => {
                write!(f, "No RPC_ENDPOINT set in .env")
            }
            ScError::NoPolkadotAddr => {
                write!(f, "No POLKADOT_ADDR set in .env")
            }
            ScError::InvalidPolkadotAddr => {
                write!(f, "Invalid POLKADOT_ADDR set in .env")
            }
            ScError::NoDataFound => {
                write!(f, "Did not find any data. Polkadot address unused?")
            }
            ScError::NoEnvFile => write!(f, "Can't find .env file."),
            ScError::IO(err) => write!(f, "Error while flushing the file {}", err),
            ScError::Reqwest(err) => write!(f, "Error while fetching data {}", err),
            ScError::Crypto(err) => write!(f, "Cryptographic error {}", err),
            ScError::Codec(err) => write!(f, "Codec error {}", err),
            ScError::Json(err) => write!(f, "Json error {}", err),
            ScError::Csv(err) => write!(f, "Comma separated value error {}", err),
        }
    }
}

impl fmt::Debug for ScError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        <Self as fmt::Display>::fmt(self, f)
    }
}

impl From<csv::Error> for ScError {
    fn from(err: csv::Error) -> ScError {
        ScError::Csv(err)
    }
}

impl From<serde_json::Error> for ScError {
    fn from(err: serde_json::Error) -> ScError {
        ScError::Json(err)
    }
}

impl From<parity_scale_codec::Error> for ScError {
    fn from(err: parity_scale_codec::Error) -> ScError {
        ScError::Codec(err)
    }
}

impl From<PublicError> for ScError {
    fn from(err: PublicError) -> ScError {
        ScError::Crypto(err)
    }
}

impl From<reqwest::Error> for ScError {
    fn from(err: reqwest::Error) -> ScError {
        ScError::Reqwest(err)
    }
}

impl From<std::io::Error> for ScError {
    fn from(err: std::io::Error) -> ScError {
        ScError::IO(err)
    }
}

pub fn known_stake_changes_file_from_env() -> String {
    match dotenv::var("KNOWN_STAKE_CHANGES_FILE") {
        Ok(s) => s,
        Err(_) => "".into(),
    }
}

pub fn known_rewards_file_from_env() -> String {
    match dotenv::var("KNOWN_REWARDS_FILE") {
        Ok(s) => s,
        Err(_) => "".into(),
    }
}

pub fn polkadot_properties_file_from_env() -> String {
    match dotenv::var("POLKADOT_PROPERTIES_FILE") {
        Ok(s) => s,
        Err(_) => "".into(),
    }
}

pub fn token_decimals(file: impl AsRef<Path>) -> Result<TokenDecimals, ScError> {
    let mut polkadot_properties: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(file)?)?;
    Ok(polkadot_properties["tokenDecimals"]
        .take()
        .as_u64()
        .unwrap_or(0) as TokenDecimals)
}

#[derive(Deserialize, PartialEq, Eq, Debug, Clone, Copy)]
pub struct StakeChange {
    #[serde(deserialize_with = "util::naive_date_time_from_str")]
    pub timestamp: NaiveDateTime,
    #[serde(deserialize_with = "util::balance_from_maybe_str")]
    #[serde(rename(deserialize = "accumulatedAmount"))]
    pub accumulated_amount: u128,
}

#[derive(Deserialize, PartialEq, Eq, Debug, Clone, Copy)]
pub struct Reward {
    #[serde(deserialize_with = "util::naive_date_time_from_str")]
    pub date: NaiveDateTime,
    #[serde(deserialize_with = "util::balance_from_maybe_str")]
    pub balance: u128,
}

impl poloto::build::unwrapper::Unwrapper for Reward {
    type Item = (NaiveDateTime, u128);
    fn unwrap(self) -> (NaiveDateTime, u128) {
        (self.date, self.balance)
    }
}

pub fn known_stake_changes(file: impl AsRef<Path>) -> Result<Vec<StakeChange>, ScError> {
    let mut stake_changes: Vec<StakeChange> = vec![];

    if let Ok(true) = &file.as_ref().try_exists() {
        let mut rdr = ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .from_path(&file)?;
        for record in rdr.deserialize() {
            let stake_change: StakeChange = record?;
            stake_changes.push(stake_change);
        }
    }
    Ok(stake_changes)
}

pub fn known_rewards(file: impl AsRef<Path>) -> Result<Vec<Reward>, ScError> {
    let mut rewards: Vec<Reward> = vec![];

    if let Ok(true) = &file.as_ref().try_exists() {
        let mut rdr = ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .from_path(&file)?;
        for record in rdr.deserialize() {
            let reward: Reward = record?;
            rewards.push(reward);
        }
    }
    Ok(rewards)
}

pub async fn get_stake_changes(
    subquery_endpoint: &str,
    polkadot_addr: &str,
    known_stake_changes_file: impl AsRef<Path>,
) -> Result<Vec<StakeChange>, ScError> {
    let olds = known_stake_changes(known_stake_changes_file)?;
    let latest = query_stake_changes(subquery_endpoint, polkadot_addr).await?;

    if let Some(newest_old) = olds.last() {
        let mut latest_iter = latest.into_iter();
        let mut news: Vec<StakeChange> = vec![];

        if latest_iter.any(|x| x.timestamp >= newest_old.timestamp) {
            for elem in latest_iter {
                news.push(elem);
            }
        }
        return Ok(news);
    }
    Ok(latest)
}

pub async fn get_staking_rewards(
    subquery_endpoint: &str,
    polkadot_addr: &str,
    known_rewards_file: impl AsRef<Path>,
) -> Result<Vec<Reward>, ScError> {
    let olds = known_rewards(known_rewards_file)?;
    let latest = query_staking_rewards(subquery_endpoint, polkadot_addr).await?;

    if let Some(newest_old) = olds.last() {
        let mut latest_iter = latest.into_iter();
        let mut news: Vec<Reward> = vec![];

        if latest_iter.any(|x| x.date >= newest_old.date) {
            for elem in latest_iter {
                news.push(elem);
            }
        }
        return Ok(news);
    }
    Ok(latest)
}

impl fmt::Display for Reward {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?},{}", self.date, self.balance)
    }
}

impl fmt::Display for StakeChange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?},{}", self.timestamp, self.accumulated_amount)
    }
}

async fn query_staking_rewards(
    subquery_endpoint: &str,
    polkadot_addr: &str,
) -> Result<Vec<Reward>, ScError> {
    let query = format!(
        "{{ stakingRewards (last: 100, orderBy: DATE_ASC, filter: \
            {{accountId : {{equalTo : \"{}\"}}}}) {{ \
              nodes {{ \
                balance \
                date \
              }}}}}}",
        polkadot_addr
    );
    let ans = util::subquery(subquery_endpoint, query).await?;
    let maybe_rewards = ans["stakingRewards"]["nodes"].as_array();
    if let Some(vec) = maybe_rewards {
        let mut ret_rewards: Vec<Reward> = Vec::new();
        for reward in vec {
            let r: Reward = serde_json::from_value(reward.clone())?;
            ret_rewards.push(r);
        }
        return Ok(ret_rewards);
    };

    Ok(vec![])
}

async fn query_stake_changes(
    subquery_endpoint: &str,
    polkadot_addr: &str,
) -> Result<Vec<StakeChange>, ScError> {
    let query = format!(
        "{{ stakeChanges (last: 100, orderBy: TIMESTAMP_ASC, filter: \
            {{address : {{equalTo: \"{}\"}}}}) {{\
              nodes {{ \
                timestamp \
                accumulatedAmount \
              }}}}}}",
        polkadot_addr
    );

    let ans = util::subquery(subquery_endpoint, query).await?;
    let maybe_stake_changes = ans["stakeChanges"]["nodes"].as_array();
    if let Some(vec) = maybe_stake_changes {
        let mut ret_stake_changes: Vec<StakeChange> = vec![];
        for stake_change in vec {
            let r: StakeChange = serde_json::from_value(stake_change.clone())?;
            ret_stake_changes.push(r);
        }
        return Ok(ret_stake_changes);
    };

    Ok(vec![])
}

pub async fn rpc_methods(rpc_endpoint: &str) -> Result<(), ScError> {
    let ans = util::rpc(rpc_endpoint, "rpc_methods", ()).await?;
    println!("{}", serde_json::to_string_pretty(&ans).unwrap());
    Ok(())
}

pub async fn state_get_metadata(rpc_endpoint: &str) -> Result<String, ScError> {
    let res = util::rpc(rpc_endpoint, "state_getMetadata", ()).await?;
    // Decode the hex value into bytes (which are the SCALE encoded metadata details):
    let metadata_hex = res.as_str().unwrap();
    let metadata_bytes = hex::decode(metadata_hex.trim_start_matches("0x")).unwrap();
    // Fortunately, we know what type the metadata is, so we are able to decode our SCALEd bytes to it:
    let decoded = RuntimeMetadataPrefixed::decode(&mut metadata_bytes.as_slice()).unwrap();
    Ok(serde_json::to_string_pretty(&decoded).unwrap())
}

pub async fn system_properties(rpc_endpoint: &str) -> Result<String, ScError> {
    let res = util::rpc(rpc_endpoint, "system_properties", ()).await?;
    Ok(serde_json::to_string_pretty(&res).unwrap())
}

pub async fn state_get_storage(
    rpc_endpoint: &str,
    module_name: &str,
    storage_name: &str,
    polkadot_addr: Option<&str>,
) -> Result<Vec<u8>, ScError> {
    let mut storage_key = Vec::new();
    storage_key.extend_from_slice(&hashing::twox_128(module_name.as_bytes()));
    storage_key.extend_from_slice(&hashing::twox_128(storage_name.as_bytes()));

    if polkadot_addr.is_some() {
        let account_id = AccountId32::from_string(polkadot_addr.unwrap()).unwrap();
        storage_key.extend_from_slice(&hashing::blake2_128(account_id.as_ref()));
        storage_key.extend_from_slice(account_id.as_ref());
    }

    let storage_key_hex = format!("0x{}", hex::encode(&storage_key));
    let result_hex = util::rpc(rpc_endpoint, "state_getStorage", (storage_key_hex,)).await?;

    let result_str = result_hex.as_str();
    if result_str.is_none() {
        return Err(ScError::NoDataFound);
    }
    let result_bytes = hex::decode(result_str.unwrap().trim_start_matches("0x")).unwrap();
    Ok(result_bytes)
}

pub fn decode_u128(mut bytes: &[u8]) -> Result<u128, ScError> {
    let res = u128::decode(&mut bytes)?;
    Ok(res)
}

pub fn stringify_encoded_u128(bytes: &[u8]) -> Result<String, ScError> {
    let res: u128 = decode_u128(bytes)?;
    Ok(format!("{}", res))
}

pub fn stringify_encoded_total_issuance(
    bytes: &[u8],
    token_decimals: TokenDecimals,
) -> Result<String, ScError> {
    let unpointed_string = stringify_encoded_u128(bytes)?;
    Ok(format!(
        "{} DOT",
        (&unpointed_string).with_decimal_point(token_decimals)
    ))
}

pub fn stringify_encoded_system_account(
    mut bytes: &[u8],
    decimals: TokenDecimals,
) -> Result<String, ScError> {
    let account_info = PolkadotAccountInfo::decode(&mut bytes)?;
    Ok(format!(
        "Nonce: {}, Consumers: {}, Providers: {}, Sufficients: {}, Free: {} DOT, Reserved: {} DOT, Misc Frozen: {} DOT, Fee Frozen: {} DOT",
        account_info.nonce,
        account_info.consumers,
        account_info.providers,
        account_info.sufficients,
        account_info.data.free.with_decimal_point(decimals),
        account_info.data.reserved.with_decimal_point(decimals),
        account_info.data.misc_frozen.with_decimal_point(decimals),
        account_info.data.fee_frozen.with_decimal_point(decimals)
    ))
}

pub async fn get_total_issuance(rpc_endpoint: &str) -> Result<u128, ScError> {
    let result_bytes = state_get_storage(rpc_endpoint, "Balances", "TotalIssuance", None).await?;
    let total_issued = decode_u128(result_bytes.as_slice())?;
    Ok(total_issued)
}

pub async fn get_account_info(
    rpc_endpoint: &str,
    polkadot_addr: &str,
) -> Result<PolkadotAccountInfo, ScError> {
    let result_bytes =
        state_get_storage(rpc_endpoint, "System", "Account", Some(polkadot_addr)).await?;
    let account_info = PolkadotAccountInfo::decode(&mut result_bytes.as_ref())?;
    Ok(account_info)
}

pub trait DecimalPointPuttable {
    fn with_decimal_point(self, decimals: TokenDecimals) -> String;
}

impl DecimalPointPuttable for &str {
    fn with_decimal_point(self, decimals: TokenDecimals) -> String {
        let len = self.chars().count();
        if len > decimals {
            let mut count = 0;
            self.chars()
                .map(|c| {
                    count += 1;
                    if count == (len - decimals) {
                        c.to_string() + "."
                    } else {
                        c.to_string()
                    }
                })
                .collect::<String>()
        } else {
            let mut pad: String = "".into();
            for _ in 0..=(decimals - len) {
                pad.push('0');
            }
            (pad + self).with_decimal_point(decimals)
        }
    }
}

impl DecimalPointPuttable for u128 {
    fn with_decimal_point(self, decimals: TokenDecimals) -> String {
        (&format!("{}", self)).with_decimal_point(decimals)
    }
}
