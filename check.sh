# read .env as bash variables
set -a
source <(cat .env | sed -e '/^#/d;/^\s*$/d' -e "s/'/'\\\''/g" -e "s/=\(.*\)/='\1'/g")
set +a
cargo run --bin stake-checker -- --staking_rewards >> $KNOWN_REWARDS_FILE && \
cargo run --bin stake-checker -- --stake_changes >> $KNOWN_STAKE_CHANGES_FILE && \
cargo run --bin plotit > plot.svg && \
eog plot.svg
