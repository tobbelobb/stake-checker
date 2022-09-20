use poloto::num::timestamp::UnixTime;
use poloto::prelude::*;
use stake_checker::*;

fn main() -> Result<(), ScError> {
    let timezone = &chrono::Utc;
    use chrono::TimeZone;

    let polkadot_properties_file = polkadot_properties_file_from_env();
    let token_decimals = token_decimals(polkadot_properties_file)?;

    let known_rewards_file = known_rewards_file_from_env();
    let rewards: Vec<Reward> = known_rewards(known_rewards_file)?;
    let data: &[_] = &rewards
        .iter()
        .map(|r| {
            let d = timezone.from_utc_datetime(&r.date);
            (
                UnixTime::from(d),
                (r.balance as f64) / f64::powf(10f64, token_decimals as f64),
            )
        })
        .collect::<Vec<_>>();

    let plotter = poloto::quick_fmt!(
        "Rewards",
        "Date",
        "DOT",
        poloto::build::markers([], [0.0]),
        data.iter().cloned_plot().line("")
    );
    print!("{}", poloto::disp(|w| plotter.simple_theme_dark(w)));

    Ok(())
}
