use assert_cmd::prelude::*; // Add methods on commands
use predicates::prelude::*; // Used for writing assertions
use std::process::Command; // Run programs

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
            Get account's staking rewards. Will skip those already listed in known_rewards.csv. Will
            retrieve at most 100 new rewards.

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
