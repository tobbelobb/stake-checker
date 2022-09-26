use assert_cmd::prelude::*; // Add methods on commands
use predicates::prelude::*; // Used for writing assertions
use std::fs;
use std::io::Write;
use std::process::Command;

struct TestDir {
    path: std::path::PathBuf,
}

impl TestDir {
    fn new(path: std::path::PathBuf) -> Self {
        let _ignore = fs::create_dir(&path);
        TestDir { path }
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir(&self.path);
    }
}

#[test]
fn help_text_works() -> Result<(), Box<dyn std::error::Error>> {
    let help_text = "Stake Checker 1.0
Torbjørn L. <tobben@fastmail.fm>
Check Polkadot Staking Rewards

USAGE:
    stake-checker [OPTIONS]

OPTIONS:
    -a, --account_balances
            Get account's balances

    -c, --stake_changes
            Get account's stake changes. Will skip those already listed in known stake changes file
            listen in .env. Will retrieve at most 100 new stake changes.

    -g, --get_storage <get_storage>...
            Raw state_getStorage rpc call. Provide at least two args: <method>, and <name>. Third is
            optional. The program will try to decode the value before printing, but will print raw
            bytes if the method+name combination is unknown.

    -h, --help
            Print help information

    -m, --metadata
            Call endpoint func state_getMetadata

    -p, --properties
            Call endpoint func system_properties

    -r, --rpc_methods
            Call endpoint func rpc_methods

    -s, --staking_rewards
            Get account's staking rewards. Will skip those already listed in known_rewards file
            listed in .env. Will retrieve at most 100 new rewards.

    -t, --total_issuance
            Get endpoint chain's total issuance

    -V, --version
            Print version information
";
    {
        let mut cmd = Command::cargo_bin("stake-checker")?;
        cmd.arg("-h");
        cmd.assert().stdout(predicate::str::starts_with(help_text));
    }
    let mut cmd2 = Command::cargo_bin("stake-checker")?;
    cmd2.assert().stderr(predicate::str::starts_with(help_text));

    Ok(())
}

#[test]
fn helpful_message_when_envfile_missing() -> Result<(), Box<dyn std::error::Error>> {
    let test_dir = TestDir::new(testfile::generate_name());
    let mut cmd = Command::cargo_bin("stake-checker").unwrap();
    cmd.arg("--staking_rewards").current_dir(&test_dir.path);

    let helpful_message = "Error: Can't find .env file";
    cmd.assert()
        .stderr(predicate::str::starts_with(helpful_message));

    Ok(())
}

#[test]
fn helpful_message_when_polkadot_addr_missing() -> Result<(), Box<dyn std::error::Error>> {
    let test_dir = TestDir::new(testfile::generate_name());
    let mut cmd = Command::cargo_bin("stake-checker").unwrap();
    cmd.arg("--staking_rewards").current_dir(&test_dir.path);

    let env_filename = test_dir.path.to_str().unwrap().to_owned() + "/.env";
    let mut env = std::fs::File::create(&env_filename).unwrap();
    env.write(
        "POLKADOT_ADDR=<your_address_here>\n\
         RPC_ENDPOINT=https://polkadot-rpc.dwellir.com\n\
         SUBQUERY_ENDPOINT_REWARDS=https://api.subquery.network/sq/subquery/tutorial---staking-sum\n\
         SUBQUERY_ENDPOINT_STAKE_CHANGES=https://api.subquery.network/sq/nova-wallet/nova-polkadot\n\
         KNOWN_REWARDS_FILE=known_rewards.csv\n\
         KNOWN_STAKE_CHANGES_FILE=known_stake_changes.csv\n\
         POLKADOT_PROPERTIES_FILE=polkadot_properties.json\n\
        "
        .as_bytes(),
    )
    .expect("Failed to write to tmp file");
    let _tf = testfile::from_file(&env_filename); // Takes care of deleting tmp file

    let helpful_message = "Error: Invalid POLKADOT_ADDR set in .env: <your_address_here>";
    cmd.assert()
        .stderr(predicate::str::starts_with(helpful_message));

    Ok(())
}

#[test]
fn helpful_message_when_subquery_endpoint_stake_changes_missing(
) -> Result<(), Box<dyn std::error::Error>> {
    let test_dir = TestDir::new(testfile::generate_name());
    let mut cmd = Command::cargo_bin("stake-checker").unwrap();
    cmd.arg("--staking_rewards").current_dir(&test_dir.path);

    let env_filename = test_dir.path.to_str().unwrap().to_owned() + "/.env";
    let mut env = std::fs::File::create(&env_filename).unwrap();
    env.write(
        "POLKADOT_ADDR=16ZL8yLyXv3V3L3z9ofR1ovFLziyXaN1DPq4yffMAZ9czzBD\n\
         RPC_ENDPOINT=https://polkadot-rpc.dwellir.com\n\
         SUBQUERY_ENDPOINT_REWARDS=https://api.subquery.network/sq/subquery/tutorial---staking-sum\n\
         KNOWN_REWARDS_FILE=known_rewards.csv\n\
         KNOWN_STAKE_CHANGES_FILE=known_stake_changes.csv\n\
         POLKADOT_PROPERTIES_FILE=polkadot_properties.json\n\
        "
        .as_bytes(),
    )
    .expect("Failed to write to tmp file");
    let _tf = testfile::from_file(&env_filename); // Takes care of deleting tmp file

    let helpful_message = "Error: No SUBQUERY_ENDPOINT_STAKE_CHANGES set in .env";
    cmd.assert()
        .stderr(predicate::str::starts_with(helpful_message));

    Ok(())
}

// Would call the real rpc node.
// We don't want that.
// And we don't want a big complicated simulation of an rpc node either.
// Haven't found an elegant solution yet.
//#[test]
//fn total_issued_works() -> Result<(), Box<dyn std::error::Error>> {
//    let mut cmd = Command::cargo_bin("stake-checker")?;
//    cmd.arg("--total_issuance");
//
//    cmd.assert().stdout(predicate::str::is_match(
//        "Total issued \\d\\d\\d\\d\\d\\d\\d\\d\\d\\d\\.\\d\\d\\d\\d\\d\\d\\d\\d\\d\\d DOT\n",
//    )?);
//
//    Ok(())
//}
