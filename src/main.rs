use std::fmt;
use util::rpc;

use clap::{Arg, Command};
use frame_metadata::RuntimeMetadataPrefixed;
use log::{error, info};
use log4rs;
use parity_scale_codec::Decode;
use sp_core::{crypto::AccountId32, crypto::Ss58AddressFormatRegistry, crypto::Ss58Codec, hashing};

#[derive(Debug)]
enum ScError {
    NoEndpoint,
    NoPolkadotAddr,
    IO(std::io::Error),
    Reqwest(reqwest::Error),
}

impl std::error::Error for ScError {}

impl fmt::Display for ScError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ScError::NoEndpoint => {
                write!(f, "No RPC endpoint is set via the .env variable.")
            }
            ScError::NoPolkadotAddr => {
                write!(f, "No Polkadot address is set via the .env variable.")
            }
            ScError::IO(err) => write!(f, "Error while flushing the file {}", err),
            ScError::Reqwest(err) => write!(f, "Error while fetching data {}", err),
        }
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

#[allow(dead_code)]
async fn rpc_methods(rpc_endpoint: &str) -> Result<(), ScError> {
    let ans = rpc(rpc_endpoint, "rpc_methods", ()).await?;
    println!("{}", serde_json::to_string_pretty(&ans).unwrap());
    Ok(())
}

async fn state_get_metadata(rpc_endpoint: &str) -> Result<(), ScError> {
    let res = rpc(rpc_endpoint, "state_getMetadata", ()).await?;
    // Decode the hex value into bytes (which are the SCALE encoded metadata details):
    let metadata_hex = res.as_str().unwrap();
    let metadata_bytes = hex::decode(&metadata_hex.trim_start_matches("0x")).unwrap();
    // Fortunately, we know what type the metadata is, so we are able to decode our SCALEd bytes to it:
    let decoded = RuntimeMetadataPrefixed::decode(&mut metadata_bytes.as_slice()).unwrap();
    println!("{}", serde_json::to_string_pretty(&decoded).unwrap());

    Ok(())
}

async fn state_get_storage(
    rpc_endpoint: &str,
    module_name: &str,
    storage_name: &str,
) -> Result<(), ScError> {
    let mut storage_key = Vec::new();
    storage_key.extend_from_slice(&hashing::twox_128(module_name.as_bytes()));
    storage_key.extend_from_slice(&hashing::twox_128(storage_name.as_bytes()));
    let storage_key_hex = format!("0x{}", hex::encode(&storage_key));
    let result_hex = rpc(rpc_endpoint, "state_getStorage", (storage_key_hex,)).await?;

    let result_hex_str = result_hex.as_str().unwrap();
    let result_bytes = hex::decode(result_hex_str.trim_start_matches("0x")).unwrap();
    let total_issued = u128::decode(&mut result_bytes.as_slice()).unwrap();
    println!("Total issued {total_issued}");
    Ok(())
}

async fn get_balance(rpc_endpoint: &str, polkadot_addr: &str) -> Result<(), ScError> {
    let module_name = "System";
    let storage_name = "Account";
    let account_id = AccountId32::from_string(polkadot_addr).unwrap();

    let mut storage_key = Vec::new();
    storage_key.extend_from_slice(&hashing::twox_128(module_name.as_bytes()));
    storage_key.extend_from_slice(&hashing::twox_128(storage_name.as_bytes()));
    storage_key.extend_from_slice(&hashing::twox_128(account_id.as_ref()));
    storage_key.extend_from_slice(account_id.as_ref());
    let storage_key_hex = format!("0x{}", hex::encode(&storage_key));
    let result_hex = rpc(rpc_endpoint, "state_getStorage", (storage_key_hex,)).await?;
    println!("{}", result_hex);
    panic!("");
    let result_scaled = hex::decode(result_hex.as_str().unwrap().trim_start_matches("0x")).unwrap();

    type PolkadotAccountInfo = pallet_system::AccountInfo<u32, pallet_balances::AccountData<u128>>;
    let account_info = PolkadotAccountInfo::decode(&mut result_scaled.as_ref());
    println!("{:?}", account_info);

    // account_id.to_ss58check_with_version(Ss58AddressFormatRegistry::PolkadotAccount.into())
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), ScError> {
    dotenv::dotenv().ok();
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();

    let rpc_endpoint = dotenv::var("RPC_ENDPOINT").expect("RPC endpoint not set");
    let polkadot_addr = dotenv::var("POLKADOT_ADDR").expect("Polkadot address not set");
    if rpc_endpoint.is_empty() {
        error!("Empty RPC endpoint provided! Please set one via the .env file!");
        return Err(ScError::NoEndpoint);
    }
    if polkadot_addr.is_empty() {
        error!("Empty Polkadot endpoint provided! Please set one via the .env file!");
        return Err(ScError::NoPolkadotAddr);
    }

    let matches = Command::new("Stake Checker")
        .version("1.0")
        .author("Torbjørn L. <tobben@fastmail.fm>")
        .about("Check Polkadot Staking Rewards")
        .arg(
            Arg::with_name("rpc_methods")
                .long("rpc_methods")
                .short('r')
                .takes_value(false)
                .help("Call endpoint func rpc_methods"),
        )
        .arg(
            Arg::with_name("get_metadata")
                .long("get_metadata")
                .short('m')
                .takes_value(false)
                .help("Call endpoint func state_getMetadata"),
        )
        .arg(
            Arg::with_name("total_issuance")
                .long("total_issuance")
                .short('t')
                .takes_value(false)
                .help("Get endpoint chain's total issuance"),
        )
        .arg(
            Arg::with_name("free_balance")
                .long("free_balance")
                .short('f')
                .takes_value(false)
                .help("Get account's free balance"),
        )
        .get_matches();

    if matches.is_present("rpc_methods") {
        return rpc_methods(&rpc_endpoint).await;
    }
    if matches.is_present("get_metadata") {
        return state_get_metadata(&rpc_endpoint).await;
    }
    if matches.is_present("total_issuance") {
        return state_get_storage(&rpc_endpoint, "Balances", "TotalIssuance").await;
    }
    if matches.is_present("free_balance") {
        return get_balance(&rpc_endpoint, &polkadot_addr).await;
    }
    Ok(())
}
