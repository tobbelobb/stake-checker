use clap::{AppSettings, Arg, Command};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use sp_core::crypto::{AccountId32, Ss58AddressFormatRegistry, Ss58Codec};

use stake_checker::*;

fn get_valid_env_var(var_name: &str) -> Result<String, ScError> {
    let var = match dotenv::var(&var_name) {
        Ok(s) => s,
        Err(_) => "".into(),
    };

    if var.is_empty() {
        return Err(ScError::MissingEnvVariable(var_name.into()));
    }

    Ok(var)
}

fn valid_subquery_endpoint_stake_changes_from_env() -> Result<String, ScError> {
    get_valid_env_var("SUBQUERY_ENDPOINT_STAKE_CHANGES")
}

fn valid_subquery_endpoint_rewards_from_env() -> Result<String, ScError> {
    get_valid_env_var("SUBQUERY_ENDPOINT_REWARDS")
}

fn valid_rpc_endpoint_from_env() -> Result<String, ScError> {
    get_valid_env_var("RPC_ENDPOINT")
}

fn valid_polkadot_addr_from_env() -> Result<String, ScError> {
    let addr = get_valid_env_var("POLKADOT_ADDR")?;
    let account_id =
        AccountId32::from_string(&addr).map_err(|_| ScError::InvalidPolkadotAddr(addr.clone()))?;
    let back_to_string =
        account_id.to_ss58check_with_version(Ss58AddressFormatRegistry::PolkadotAccount.into());
    if addr != back_to_string {
        return Err(ScError::InvalidPolkadotAddr(addr));
    }
    Ok(addr)
}

#[tokio::main]
async fn main() -> Result<(), ScError> {
    let matches = Command::new("Stake Checker")
        .version("1.0")
        .author("Torbjørn L. <tobben@fastmail.fm>")
        .about("Check Polkadot Staking Rewards")
        .setting(AppSettings::ArgRequiredElseHelp)
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
            Arg::with_name("stake_changes")
                .long("stake_changes")
                .short('c')
                .takes_value(false)
                .help(
                    "Get account's stake changes. \
                       Will skip those already listed in \
                       known stake changes file listen in .env. \
                       Will retrieve at most 100 new stake changes.",
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
                       known_rewards file listed in .env. \
                       Will retrieve at most 100 new rewards.",
                ),
        )
        .get_matches();

    match dotenv::dotenv() {
        Ok(_) => (),
        Err(_) => return Err(ScError::NoEnvFile),
    }

    let rpc_endpoint = valid_rpc_endpoint_from_env()?;
    let subquery_endpoint_rewards = valid_subquery_endpoint_rewards_from_env()?;
    let subquery_endpoint_stake_changes = valid_subquery_endpoint_stake_changes_from_env()?;
    let polkadot_addr = valid_polkadot_addr_from_env()?;
    let known_rewards_file = known_rewards_file_from_env();
    let known_stake_changes_file = known_stake_changes_file_from_env();
    let polkadot_properties_file = polkadot_properties_file_from_env();

    match Path::new(&polkadot_properties_file).try_exists() {
        Ok(true) => (),
        _ => {
            eprintln!("Couldn't find {polkadot_properties_file}. Creating and populating it.");
            let polka_props = system_properties(&rpc_endpoint).await?;
            fs::write(&polkadot_properties_file, polka_props).expect("Unable to write file");
        }
    };
    let token_decimals = token_decimals(polkadot_properties_file)?;

    if matches.is_present("stake_changes") {
        let stake_changes = get_stake_changes(
            &subquery_endpoint_stake_changes,
            &polkadot_addr,
            &known_stake_changes_file,
        )
        .await?;
        print!(
            "{}",
            stake_changes
                .iter()
                .fold(String::new(), |acc, c| acc + &c.to_string() + "\n")
        );
    }
    let sr = SubqueryEndpoint::new(subquery_endpoint_rewards);
    if matches.is_present("staking_rewards") {
        let staking_rewards = get_staking_rewards(sr, &polkadot_addr, &known_rewards_file).await?;
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
        let metadata = state_get_metadata(&rpc_endpoint).await?;
        println!("{metadata}");
    }
    if matches.is_present("properties") {
        let sys_props = system_properties(&rpc_endpoint).await?;
        println!("{sys_props}");
    }
    if matches.is_present("total_issuance") {
        let total_issuance = get_total_issuance(&rpc_endpoint).await?;
        println!(
            "Total issued {} DOT",
            total_issuance.with_decimal_point(token_decimals)
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
                let stringified = stringify(bytes.as_slice(), token_decimals)?;
                println!("{stringified}");
            }
            None => println!("{:?}", bytes),
        }
    }

    Ok(())
}
