# read .env as bash variables
set -a
source <(cat .env | sed -e '/^#/d;/^\s*$/d' -e "s/'/'\\\''/g" -e "s/=\(.*\)/='\1'/g")
set +a
rm $KNOWN_REWARDS_FILE $KNOWN_STAKE_CHANGES_FILE
