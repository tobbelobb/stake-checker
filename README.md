## stake-checker

A program that interacts with a Polkadot rpc node.

### Usage

Configure the url of the rpc node, and the polkadot address in an .env file:
```bash
echo "POLKADOT_ADDR=<your_address_here>" >> .env
echo "RPC_ENDPOINT=https://polkadot-rpc.dwellir.com" >> .env
echo "SUBQUERY_ENDPOINT=https://api.subquery.network/sq/subquery/tutorial---staking-sum" >> .env
echo "KNOWN_REWARDS_FILE=known_rewards.csv" >> .env
```

Build the binary and ask what it can do for you:
```bash
cargo run -- --help
```

Do an example rpc query for total issuance on polkadot
```bash
cargo run -- --total_issuance
```

Make the same query but by providing a storage method and a storage name
```bash
cargo run -- --get_storage Balances TotalIssuance
```

Ask the subquery endpoint for a list of your latest staking rewards that were not already listed among your known rewards, and append them onto your file of known rewards
```bash
cargo run -- --staking_rewards >> known_rewards.csv
```


### Reading List
 - [Querying Substrate Storage Via rpc](https://www.shawntabrizi.com/substrate/querying-substrate-storage-via-rpc/)
 - [Transparent Keys in Substrate](https://www.shawntabrizi.com/substrate/transparent-keys-in-substrate/)
 - [Polkadot Interaction Examples rs](https://github.com/paritytech/polkadot-interaction-examples-rs)
 - [subxt: A Library to Submit Extrinsics to a Substrate Node via RPC](https://github.com/paritytech/subxt)
 - [substrate-api-client: A Rust Lib for Connecting to Substrate RPC Interface via WebSockets](https://github.com/scs/substrate-api-client)
 - [SubQuery Staking Sum Tutorial Example](https://explorer.subquery.network/subquery/subquery/tutorial---staking-sum)
