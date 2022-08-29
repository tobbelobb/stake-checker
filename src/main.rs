use std::fmt;
use util::rpc;

use clap::Command;
use log::error;
use log4rs;

#[derive(Debug)]
enum OneError {
    NoEndpoint,
    NoPolkadotAddr,
    CSV(csv::Error),
    IO(std::io::Error),
    Reqwest(reqwest::Error),
}

impl std::error::Error for OneError {}

impl fmt::Display for OneError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            OneError::NoEndpoint => write!(f, "No RPC endpoint is set via the .env variable."),
            OneError::NoPolkadotAddr => {
                write!(f, "No Polkadot address is set via the .env variable.")
            }
            OneError::CSV(err) => write!(f, "Error while writing the CSV file {}", err),
            OneError::IO(err) => write!(f, "Error while flushing the file {}", err),
            OneError::Reqwest(err) => write!(f, "Error while fetching data {}", err),
        }
    }
}

impl From<reqwest::Error> for OneError {
    fn from(err: reqwest::Error) -> OneError {
        OneError::Reqwest(err)
    }
}

impl From<csv::Error> for OneError {
    fn from(err: csv::Error) -> OneError {
        OneError::CSV(err)
    }
}

impl From<std::io::Error> for OneError {
    fn from(err: std::io::Error) -> OneError {
        OneError::IO(err)
    }
}

async fn rpc_methods(rpc_endpoint: &str) -> Result<(), OneError> {
    let ans = rpc(rpc_endpoint, "rpc_methods", ()).await?;
    println!("{}", serde_json::to_string_pretty(&ans).unwrap());
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), OneError> {
    dotenv::dotenv().ok();
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();

    let _matches = Command::new("Stake Checker")
        .version("1.0")
        .author("Torbjørn L. <tobben@fastmail.fm>")
        .about("Get Polkadot Staking Rewards");

    let rpc_endpoint = dotenv::var("RPC_ENDPOINT").expect("RPC endpoint not set");
    let polkadot_addr = dotenv::var("POLKADOT_ADDR").expect("Polkadot address not set");
    if rpc_endpoint.is_empty() {
        error!("Empty RPC endpoint provided! Please set one via the .env file!");
        return Err(OneError::NoEndpoint);
    }
    if polkadot_addr.is_empty() {
        error!("Empty Polkadot endpoint provided! Please set one via the .env file!");
        return Err(OneError::NoPolkadotAddr);
    }

    rpc_methods(&rpc_endpoint)
}
