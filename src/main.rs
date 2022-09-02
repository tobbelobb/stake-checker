use std::collections::HashMap;
use std::fmt;
use util::rpc;

use clap::{Arg, Command};
use frame_metadata::RuntimeMetadataPrefixed;
use log::error;
use log4rs;
use parity_scale_codec::Decode;
use sp_core::{
    crypto::AccountId32, crypto::PublicError, crypto::Ss58AddressFormatRegistry, crypto::Ss58Codec,
    hashing,
};

#[cfg(test)]
mod tests;

type PolkadotAccountInfo = pallet_system::AccountInfo<u32, pallet_balances::AccountData<u128>>;
type Stringifier = fn(Vec<u8>) -> Result<String, ScError>;

#[derive(Debug)]
enum ScError {
    NoEndpoint,
    NoPolkadotAddr,
    InvalidPolkadotAddr,
    NoDataFound,
    IO(std::io::Error),
    Reqwest(reqwest::Error),
    Crypto(PublicError),
    Codec(parity_scale_codec::Error),
}

impl std::error::Error for ScError {}

impl fmt::Display for ScError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ScError::NoEndpoint => {
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
            ScError::IO(err) => write!(f, "Error while flushing the file {}", err),
            ScError::Reqwest(err) => write!(f, "Error while fetching data {}", err),
            ScError::Crypto(err) => write!(f, "Cryptographic error {}", err),
            ScError::Codec(err) => write!(f, "Codec error {}", err),
        }
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

#[allow(dead_code)]
async fn rpc_methods(rpc_endpoint: &str) -> Result<(), ScError> {
    let ans = rpc(rpc_endpoint, "rpc_methods", ()).await?;
    println!("{}", serde_json::to_string_pretty(&ans).unwrap());
    Ok(())
}

async fn state_get_metadata(rpc_endpoint: &str) -> Result<String, ScError> {
    let res = rpc(rpc_endpoint, "state_getMetadata", ()).await?;
    // Decode the hex value into bytes (which are the SCALE encoded metadata details):
    let metadata_hex = res.as_str().unwrap();
    let metadata_bytes = hex::decode(&metadata_hex.trim_start_matches("0x")).unwrap();
    // Fortunately, we know what type the metadata is, so we are able to decode our SCALEd bytes to it:
    let decoded = RuntimeMetadataPrefixed::decode(&mut metadata_bytes.as_slice()).unwrap();
    Ok(serde_json::to_string_pretty(&decoded).unwrap())
}

async fn state_get_storage(
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
    let result_hex = rpc(rpc_endpoint, "state_getStorage", (storage_key_hex,)).await?;

    let result_str = result_hex.as_str();
    if result_str.is_none() {
        return Err(ScError::NoDataFound);
    }
    let result_bytes = hex::decode(result_str.unwrap().trim_start_matches("0x")).unwrap();
    Ok(result_bytes)
}

fn decode_u128(bytes: Vec<u8>) -> Result<u128, ScError> {
    let res = u128::decode(&mut bytes.as_slice())?;
    Ok(res)
}

fn stringify_encoded_u128(bytes: Vec<u8>) -> Result<String, ScError> {
    let res: u128 = decode_u128(bytes)?;
    Ok(format!("{}", res))
}

fn stringify_encoded_total_issuance(bytes: Vec<u8>) -> Result<String, ScError> {
    let unpointed_string = stringify_encoded_u128(bytes)?;
    Ok(format!("{} DOT", (&unpointed_string).with_decimal_point()))
}

fn stringify_encoded_system_account(bytes: Vec<u8>) -> Result<String, ScError> {
    let account_info = PolkadotAccountInfo::decode(&mut bytes.as_ref())?;
    Ok(format!(
        "Nonce: {}, Consumers: {}, Providers: {}, Sufficients: {}, Free: {} DOT, Reserved: {} DOT, Misc Frozen: {} DOT, Fee Frozen: {} DOT",
        account_info.nonce,
        account_info.consumers,
        account_info.providers,
        account_info.sufficients,
        account_info.data.free.with_decimal_point(),
        account_info.data.reserved.with_decimal_point(),
        account_info.data.misc_frozen.with_decimal_point(),
        account_info.data.fee_frozen.with_decimal_point()
    ))
}

async fn get_total_issuance(rpc_endpoint: &str) -> Result<u128, ScError> {
    let result_bytes = state_get_storage(rpc_endpoint, "Balances", "TotalIssuance", None).await?;
    let total_issued = decode_u128(result_bytes)?;
    Ok(total_issued)
}

async fn get_account_info(
    rpc_endpoint: &str,
    polkadot_addr: &str,
) -> Result<PolkadotAccountInfo, ScError> {
    let result_bytes =
        state_get_storage(rpc_endpoint, "System", "Account", Some(polkadot_addr)).await?;
    let account_info = PolkadotAccountInfo::decode(&mut result_bytes.as_ref())?;
    Ok(account_info)
}

fn valid_rpc_endpoint_from_env() -> Result<String, ScError> {
    let rpc_endpoint = match dotenv::var("RPC_ENDPOINT") {
        Ok(s) => s,
        Err(_) => "".into(),
    };

    if rpc_endpoint.is_empty() {
        error!("Empty RPC_ENDPOINT provided in .env file.");
        return Err(ScError::NoEndpoint);
    }

    Ok(rpc_endpoint)
}

fn valid_polkadot_addr_from_env() -> Result<String, ScError> {
    let addr = match dotenv::var("POLKADOT_ADDR") {
        Ok(s) => s,
        Err(_) => "".into(),
    };

    if addr.is_empty() {
        error!("Empty POLKADOT_ADDR provided in .env file.");
        return Err(ScError::NoPolkadotAddr);
    }
    let account_id = AccountId32::from_string(&addr)?;
    let back_to_string =
        account_id.to_ss58check_with_version(Ss58AddressFormatRegistry::PolkadotAccount.into());
    if addr != back_to_string {
        error!("Invalid POLKADOT_ADDR provided in .env file.");
        return Err(ScError::InvalidPolkadotAddr);
    }
    Ok(addr)
}

trait DecimalPointPuttable {
    fn with_decimal_point(self) -> String;
}

impl DecimalPointPuttable for &str {
    fn with_decimal_point(self) -> String {
        const POLKADOT_DECIMAL_PLACES: usize = 10;
        let len = self.chars().count();
        if len > POLKADOT_DECIMAL_PLACES {
            let mut count = 0;
            self.chars()
                .map(|c| {
                    count = count + 1;
                    if count == (len - POLKADOT_DECIMAL_PLACES) {
                        return c.to_string() + ".";
                    } else {
                        return c.to_string();
                    }
                })
                .collect::<String>()
        } else {
            let mut pad: String = "".into();
            for _ in 0..=(POLKADOT_DECIMAL_PLACES - len) {
                pad.push('0');
            }
            (pad + self).with_decimal_point()
        }
    }
}

impl DecimalPointPuttable for u128 {
    fn with_decimal_point(self) -> String {
        (&format!("{}", self)).with_decimal_point()
    }
}

#[tokio::main]
async fn main() -> Result<(), ScError> {
    dotenv::dotenv().ok();
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();

    let rpc_endpoint = valid_rpc_endpoint_from_env()?;
    let polkadot_addr = valid_polkadot_addr_from_env()?;

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
            Arg::with_name("account_info")
                .long("account_info")
                .short('a')
                .takes_value(false)
                .help("Get account's free balance"),
        )
        .arg(
            Arg::with_name("get_storage")
                .long("get_storage")
                .short('g')
                .takes_value(true)
                .multiple_values(true)
                .min_values(2)
                .max_values(3)
                .help("Raw state_getStorage call to the endpoint. Provide at least two arguments: <method>, and <name>. Third argument is optional. The program will try to decode the value before printing, but will print raw bytes if the method+name combination is unknown."),
        )
        .arg(
            Arg::with_name("test")
                .long("test")
                .takes_value(false)
                .help("Used for development purposes"),
        )
        .get_matches();

    if matches.is_present("rpc_methods") {
        return rpc_methods(&rpc_endpoint).await;
    }
    if matches.is_present("get_metadata") {
        let metadata = state_get_metadata(&rpc_endpoint).await?;
        println!("{}", metadata);
    }
    if matches.is_present("total_issuance") {
        let total_issuance = get_total_issuance(&rpc_endpoint).await?;
        println!(
            "Total issued {} DOT",
            (&total_issuance.to_string()).with_decimal_point()
        );
    }
    if matches.is_present("account_info") {
        println!(
            "{:?}",
            get_account_info(&rpc_endpoint, &polkadot_addr).await?
        );
    }
    if matches.is_present("get_storage") {
        let mut known_stringifiers = HashMap::<String, Stringifier>::new();
        known_stringifiers.insert(
            "BalancesTotalIssuance".into(),
            stringify_encoded_total_issuance,
        );
        known_stringifiers.insert("SystemAccount".into(), stringify_encoded_system_account);
        let args: Vec<_> = matches
            .get_many::<String>("get_storage")
            .expect("Storage module and name are required")
            .collect();

        let key = args[0].to_owned() + args[1];
        let bytes = match args.len() {
            2 => state_get_storage(&rpc_endpoint, args[0], args[1], None).await?,
            3 => state_get_storage(&rpc_endpoint, args[0], args[1], Some(args[2])).await?,
            _ => unreachable!(),
        };

        let stringifier = known_stringifiers.get(&key);
        match stringifier {
            Some(stringify) => {
                println!("Using formatter");
                println!("{}", stringify(bytes)?);
            }
            None => println!("{:?}", bytes),
        }
    }
    if matches.is_present("test") {
        return Ok(());
    }

    Ok(())
}
