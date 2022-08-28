use std::collections::HashMap;
use std::fmt;

use clap::{App, Arg};
//use csv::Writer;
use log::{debug, error, info};
use log4rs;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Serialize, Deserialize, Debug)]
struct CMCResponse {
    data: HashMap<String, Currency>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Currency {
    name: String,
    symbol: String,
    quote: Quotes,
}

#[derive(Serialize, Deserialize, Debug)]
struct Quotes(HashMap<String, Quote>);

#[derive(Serialize, Deserialize, Debug)]
struct Quote {
    price: f64,
    percent_change_7d: f64,
}

#[derive(Debug)]
enum OneError {
    NoAPIKey,
    CSV(csv::Error),
    IO(std::io::Error),
    Reqwest(reqwest::Error),
}

impl std::error::Error for OneError {}

impl fmt::Display for OneError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            OneError::NoAPIKey => write!(f, "No API key is set via the .env variable."),
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

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Name: {}, Symbol: {} Price: {} Change(7d): {}%",
            self.name,
            self.symbol,
            self.quote.0.get("USD").unwrap().price.to_string(),
            self.quote
                .0
                .get("USD")
                .unwrap()
                .percent_change_7d
                .to_string()
        )
    }
}

impl CMCResponse {
    fn get_currency(&self, currency: &str) -> Option<&Currency> {
        self.data.get(currency)
    }
}

#[tokio::main]
async fn main() -> Result<(), OneError> {
    dotenv::dotenv().ok();
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();

    let matches = App::new("Stake Checker")
        .version("1.0")
        .author("Torbjørn L. <tobben@fastmail.fm>")
        .about("Rust Training Project based on onetutorial by Bastian G")
        .arg(
            Arg::new("currency_list")
                .long("currencies")
                .short('c')
                .help("Pass the list of currencies you want to query")
                .min_values(1)
                .required(true),
        )
        .get_matches();

    let currency_list = matches
        .value_of("currency_list")
        .expect("No currencies were being passed");
    //        .collect::<Vec<_>>();
    debug!("Querying the following currencies: {:?}", currency_list);

    //for item in matches
    //    .get_many::<String>("currency_list")
    //    .expect("Must have currency list")
    //    .collect::<Vec<_>>()
    //{
    //    println!("{:?}, ", item);
    //}
    let cmc_pro_api_key = dotenv::var("CMC_PRO_API_KEY").expect("CMC key not set");
    if cmc_pro_api_key.is_empty() {
        error!("Empty CMC API KEY provided! Please set one via the .env file!");
        return Err(OneError::NoAPIKey);
    }

    let mut params = HashMap::new();
    params.insert("id", "1");
    params.insert("jsonrpc", "2.0");
    params.insert("method", "rpc_methods");

    let client = reqwest::Client::new();
    let resp = client
        .post("https://rpc.polkadot.io")
        .json(&json! {{
            "id": 1,
            "jsonrpc": "2.0",
            "method": "rpc_methods",
        }})
        .send()
        .await?;

    let ans: Value = resp.json().await?;
    println!("{}", serde_json::to_string_pretty(&ans).unwrap());

    //let currencies = resp.json::<CMCResponse>().await?;
    //let mut wtr = Writer::from_path("prices.csv")?;
    //wtr.write_record(&["Name", "Symbol", "Price", "7DayChange"])?;

    //for (symbol, currency) in currencies.data.into_iter() {
    //    wtr.write_record(&[
    //        currency.name,
    //        symbol.to_owned(),
    //        currency.quote.0.get("USD").unwrap().price.to_string(),
    //        currency
    //            .quote
    //            .0
    //            .get("USD")
    //            .unwrap()
    //            .percent_change_7d
    //            .to_string(),
    //    ])?;
    //}
    //wtr.flush()?;

    //info!("Queried {} and wrote CSV file", currency_list);

    Ok(())
}
