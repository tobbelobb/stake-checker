#![feature(fs_try_exists)]
use clap::{Arg, Command};
use log::error;
use std::collections::HashMap;
use std::fs;

use sp_core::crypto::{AccountId32, Ss58AddressFormatRegistry, Ss58Codec};

use stake_checker::*;

fn get_valid_env_var(var_name: &str, err: ScError) -> Result<String, ScError> {
    let var = match dotenv::var(var_name) {
        Ok(s) => s,
        Err(_) => "".into(),
    };

    if var.is_empty() {
        error!("No {var_name} provided in .env file.");
        return Err(err);
    }

    Ok(var)
}

fn valid_subquery_endpoint_from_env() -> Result<String, ScError> {
    get_valid_env_var("SUBQUERY_ENDPOINT", ScError::NoSubqueryEndpoint)
}

fn valid_rpc_endpoint_from_env() -> Result<String, ScError> {
    get_valid_env_var("RPC_ENDPOINT", ScError::NoRpcEndpoint)
}

fn valid_polkadot_addr_from_env() -> Result<String, ScError> {
    let addr = get_valid_env_var("POLKADOT_ADDR", ScError::NoPolkadotAddr)?;
    let account_id = AccountId32::from_string(&addr)?;
    let back_to_string =
        account_id.to_ss58check_with_version(Ss58AddressFormatRegistry::PolkadotAccount.into());
    if addr != back_to_string {
        error!("Invalid POLKADOT_ADDR provided in .env file.");
        return Err(ScError::InvalidPolkadotAddr);
    }
    Ok(addr)
}

#[tokio::main]
async fn main() -> Result<(), ScError> {
    let matches = Command::new("Stake Checker")
        .version("1.0")
        .author("Torbjørn L. <tobben@fastmail.fm>")
        .about("Check Polkadot Staking Rewards")
        .setting(clap::AppSettings::ArgRequiredElseHelp)
        .arg(
            Arg::with_name("rpc_methods")
                .long("rpc_methods")
                .short('r')
                .takes_value(false)
                .help("Call endpoint func rpc_methods"),
        )
        .arg(
            Arg::with_name("metadata")
                .long("metadata")
                .short('m')
                .takes_value(false)
                .help("Call endpoint func state_getMetadata"),
        )
        .arg(
            Arg::with_name("properties")
                .long("properties")
                .short('p')
                .takes_value(false)
                .help("Call endpoint func system_properties"),
        )
        .arg(
            Arg::with_name("total_issuance")
                .long("total_issuance")
                .short('t')
                .takes_value(false)
                .help("Get endpoint chain's total issuance"),
        )
        .arg(
            Arg::with_name("account_balances")
                .long("account_balances")
                .short('a')
                .takes_value(false)
                .help("Get account's balances"),
        )
        .arg(
            Arg::with_name("get_storage")
                .long("get_storage")
                .short('g')
                .takes_value(true)
                .multiple_values(true)
                .min_values(2)
                .max_values(3)
                .help(
                    "Raw state_getStorage rpc call. Provide at least two args: <method>, \
                    and <name>. Third is optional. The program will try to decode the value \
                    before printing, but will print raw bytes if the method+name combination \
                    is unknown.",
                ),
        )
        .arg(
            Arg::with_name("staking_rewards")
                .long("staking_rewards")
                .short('s')
                .takes_value(false)
                .help(
                    "Get account's staking rewards. \
                       Will skip those already listed in \
                       known_rewards.csv. \
                       Will retrieve at most 100 new rewards.",
                ),
        )
        .get_matches();

    dotenv::dotenv().ok();
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();

    let rpc_endpoint = valid_rpc_endpoint_from_env()?;
    let subquery_endpoint = valid_subquery_endpoint_from_env()?;
    let polkadot_addr = valid_polkadot_addr_from_env()?;
    let known_rewards_file = known_rewards_file_from_env();
    let polkadot_properties_file = polkadot_properties_file_from_env();

    match fs::try_exists(&polkadot_properties_file) {
        Ok(true) => (),
        _ => {
            println!("Couldn't find {polkadot_properties_file}. Creating and populating it.");
            fs::write(
                &polkadot_properties_file,
                system_properties(&rpc_endpoint).await?,
            )
            .expect("Unable to write file");
        }
    };
    let token_decimals = token_decimals(polkadot_properties_file)?;

    if matches.is_present("staking_rewards") {
        let staking_rewards =
            get_staking_rewards(&subquery_endpoint, &polkadot_addr, &known_rewards_file).await?;
        print!(
            "{}",
            staking_rewards
                .iter()
                .fold(String::new(), |acc, r| acc + &r.to_string() + "\n")
        );
    }
    if matches.is_present("rpc_methods") {
        return rpc_methods(&rpc_endpoint).await;
    }
    if matches.is_present("metadata") {
        println!("{}", state_get_metadata(&rpc_endpoint).await?);
    }
    if matches.is_present("properties") {
        println!("{}", system_properties(&rpc_endpoint).await?);
    }
    if matches.is_present("total_issuance") {
        let total_issuance = get_total_issuance(&rpc_endpoint).await?;
        println!(
            "Total issued {} DOT",
            (&total_issuance.to_string()).with_decimal_point(token_decimals)
        );
    }
    if matches.is_present("account_balances") {
        let account_info = get_account_info(&rpc_endpoint, &polkadot_addr).await?;
        println!(
            "Free: {} DOT, Reserved: {} DOT, Misc Frozen: {} DOT, Fee Frozen: {} DOT",
            account_info.data.free.with_decimal_point(token_decimals),
            account_info
                .data
                .reserved
                .with_decimal_point(token_decimals),
            account_info
                .data
                .misc_frozen
                .with_decimal_point(token_decimals),
            account_info
                .data
                .fee_frozen
                .with_decimal_point(token_decimals)
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
                println!("{}", stringify(bytes.as_slice(), token_decimals)?);
            }
            None => println!("{:?}", bytes),
        }
    }

    Ok(())
}
