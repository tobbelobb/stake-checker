use poloto::num::timestamp::UnixTime;
use poloto::prelude::*;
use stake_checker::*;

fn main() -> Result<(), ScError> {
    let timezone = &chrono::Utc;
    use chrono::TimeZone;

    let polkadot_properties_file = polkadot_properties_file_from_env();
    let token_decimals = token_decimals(polkadot_properties_file)?;

    let known_rewards_file = known_rewards_file_from_env();
    let rewards = known_rewards(known_rewards_file)?;

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

    let mut rewards_time_averaged: Vec<Reward> = vec![];
    let mut date_time = rewards.first().unwrap().date;
    let window_days = 8;
    let mut rew_iter = rewards.iter();
    let mut skip_samples = 0;
    while date_time <= rewards.last().unwrap().date {
        date_time += chrono::Duration::days(1);
        let time_window = [
            date_time
                .checked_add_signed(chrono::Duration::days(-window_days))
                .unwrap(),
            date_time,
        ];

        for x in rew_iter.by_ref() {
            if x.date >= time_window[0] {
                break;
            }
            skip_samples += 1;
        }
        let left_pos = rewards
            .iter()
            .skip(skip_samples)
            .position(|x| x.date > time_window[0])
            .unwrap_or(0);
        let right_pos = rewards
            .iter()
            .skip(skip_samples)
            .position(|x| x.date >= time_window[1])
            .unwrap_or(rewards.len());
        let sum = rewards[left_pos..right_pos]
            .iter()
            .fold(0, |acc, x| acc + x.balance)
            / (window_days as u128);
        rewards_time_averaged.push(Reward {
            date: date_time,
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

    let dates_w_dummys = rewards_w_dummys.iter().map(|r| {
        let d = timezone.from_utc_datetime(&r.date);
        UnixTime::from(d)
    });
    let balances_w_dummys = rewards_w_dummys
        .iter()
        .map(|r| (r.balance as f64) / f64::powf(10f64, token_decimals as f64))
        .collect::<Vec<_>>();
    let data_w_dummys = dates_w_dummys.zip(balances_w_dummys);

    let data_for_plot = poloto::data(plots!(
        data_w_dummys.buffered_plot().histogram("Payouts"),
        data_time_averaged
            .buffered_plot()
            .line(format!("SMA {window_days} days"))
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
