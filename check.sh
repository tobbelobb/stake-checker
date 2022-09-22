cargo run --bin stake-checker -- --staking_rewards >> known_rewards.csv && \
cargo run --bin stake-checker -- --stake_changes >> known_stake_changes.csv && \
cargo run --bin plotit > plot.svg && \
eog plot.svg
