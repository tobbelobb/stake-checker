use poloto::num::timestamp::UnixTime;
use poloto::prelude::*;
use stake_checker::*;
use std::cmp::min;

fn main() -> Result<(), ScError> {
    let timezone = &chrono::Utc;
    use chrono::TimeZone;

    let polkadot_properties_file = polkadot_properties_file_from_env();
    let token_decimals = token_decimals(polkadot_properties_file)?;

    let known_rewards_file = known_rewards_file_from_env();
    let rewards = known_rewards(known_rewards_file)?;
    let known_stake_changes_file = known_stake_changes_file_from_env();
    let stake_changes = known_stake_changes(known_stake_changes_file)?;

    // Build the expected reward data set
    let mut stake_changes_w_dummys: Vec<StakeChange> = vec![];
    // Add dummy stake changes to get near vertical expectation increases when
    // stake increases sharply
    let mut it = stake_changes.iter().peekable();
    while let Some(stake_change) = it.next() {
        stake_changes_w_dummys.push(*stake_change);
        stake_changes_w_dummys.push(StakeChange {
            timestamp: it
                .peek()
                .unwrap_or(&stake_change)
                .timestamp
                .checked_add_signed(chrono::Duration::hours(-1))
                .unwrap_or(stake_change.timestamp),
            accumulated_amount: stake_change.accumulated_amount,
        });
    }
    let dates_expected_rewards = stake_changes_w_dummys.iter().map(|r| {
        let d = timezone.from_utc_datetime(&r.timestamp);
        UnixTime::from(d)
    });

    const EXPECTED_APR: f64 = 0.14;
    let expected_rewards = stake_changes_w_dummys
        .iter()
        .map(|c| {
            // Shamelessly hard-coded from https://www.stakingrewards.com/earn/polkadot/
            let daily_growth_factor: f64 = EXPECTED_APR / 365f64;
            let dots = (c.accumulated_amount as f64) / f64::powf(10f64, token_decimals as f64);
            dots * daily_growth_factor
        })
        .collect::<Vec<_>>();
    let data_expected_rewards = dates_expected_rewards.clone().zip(expected_rewards);

    // Add dummy data to rewards to get uniform width histogram staples
    let mut rewards_w_dummys: Vec<Reward> = vec![];
    for reward in &rewards {
        rewards_w_dummys.push(*reward);
        rewards_w_dummys.push(Reward {
            date: reward
                .date
                .checked_add_signed(chrono::Duration::hours(1))
                .unwrap_or(reward.date),
            balance: 0,
        });
    }
    let dates_w_dummys = rewards_w_dummys.iter().map(|r| {
        let d = timezone.from_utc_datetime(&r.date);
        UnixTime::from(d)
    });
    let balances_w_dummys = rewards_w_dummys
        .iter()
        .map(|r| (r.balance as f64) / f64::powf(10f64, token_decimals as f64))
        .collect::<Vec<_>>();
    let data_w_dummys = dates_w_dummys.zip(balances_w_dummys);

    // Build time averaged rewards data set
    let window_step_interval = chrono::Duration::days(1);
    let window_steps = 14;
    let window_length = window_step_interval * window_steps;
    let mut window_start = rewards.first().unwrap().date;
    let mut window_end = window_start.checked_add_signed(window_length).unwrap();
    let mut skip_samples = 0;
    let mut rewards_time_averaged: Vec<Reward> = vec![];
    while window_end <= rewards.last().unwrap().date {
        window_start += window_step_interval;
        window_end += window_step_interval;

        skip_samples += rewards[skip_samples..]
            .iter()
            .position(|x| x.date > window_start)
            .unwrap_or(0);

        let right_pos = rewards[skip_samples..]
            .iter()
            .position(|x| x.date >= window_end)
            .unwrap_or(rewards.len());
        let sum = rewards[skip_samples..min(right_pos + skip_samples, rewards.len())]
            .iter()
            .fold(0, |acc, x| acc + x.balance)
            / (window_steps as u128);
        rewards_time_averaged.push(Reward {
            date: window_end,
            balance: sum,
        });
    }
    let dates_time_averaged = rewards_time_averaged.iter().map(|r| {
        let d = timezone.from_utc_datetime(&r.date);
        UnixTime::from(d)
    });
    let balances_time_averaged = rewards_time_averaged
        .iter()
        .map(|r| (r.balance as f64) / f64::powf(10f64, token_decimals as f64))
        .collect::<Vec<_>>();
    let data_time_averaged = dates_time_averaged.zip(balances_time_averaged);

    let data_for_plot = poloto::data(plots!(
        data_w_dummys.buffered_plot().histogram("Payouts"),
        data_time_averaged
            .buffered_plot()
            .line(format!("SMA {window_steps} days")),
        data_expected_rewards
            .buffered_plot()
            .line(format!("{:.1}% APR", EXPECTED_APR * 100.))
    ));

    let plotting_area_size = [1500.0, 800.0];
    let opt = poloto::render::render_opt_builder()
        .with_tick_lines([false, false])
        .with_dim(plotting_area_size)
        .build();
    let (bx, by) = poloto::ticks::bounds(&data_for_plot, &opt);
    let xtick_fmt = poloto::ticks::from_default(bx);
    let ytick_fmt = poloto::ticks::from_default(by);

    let plotter = poloto::plot_with(
        data_for_plot,
        opt,
        poloto::plot_fmt("Rewards", "", "DOT", xtick_fmt, ytick_fmt),
    );
    print!("{}", poloto::disp(|w| plotter.simple_theme_dark(w)));

    Ok(())
}
