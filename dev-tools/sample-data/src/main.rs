mod sample;

use std::env;

use anyhow::{Context, bail};
use mmp_server::bootstrap;
use time::macros::format_description;
use time::{Date, Duration, OffsetDateTime, Weekday};

use crate::sample::{Scenario, load};

struct Args {
    scenario: Scenario,
    week_start: Date,
    today: Date,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if env::var("MMP_SAMPLE_DATA").as_deref() != Ok("true") {
        bail!("sample data is disabled; run it through the development Compose service");
    }

    let args = parse_args()?;
    bootstrap::init_tracing();
    let config = bootstrap::load_config()?;
    let pool = bootstrap::connect_and_migrate(&config).await?;
    let state = bootstrap::app_state(&config, &pool);

    bootstrap::ensure_bootstrap_user(&state.household, &config.dev_user).await?;
    bootstrap::apply_seed(&state.catalogue).await?;

    let report = load(
        &state,
        &config.dev_user,
        args.scenario,
        args.week_start,
        args.today,
    )
    .await?;
    tracing::info!(
        scenario = %args.scenario,
        week_start = %args.week_start,
        today = %args.today,
        users_created = report.users_created,
        members_created = report.members_created,
        products_created = report.products_created,
        recipes_created = report.recipes_created,
        targets_created = report.targets_created,
        stock_items_created = report.stock_items_created,
        meals_created = report.meals_created,
        meals_resolved = report.meals_resolved,
        stock_effects_applied = report.stock_effects_applied,
        household_participants_created = report.household_participants_created,
        diary_entries_created = report.diary_entries_created,
        "sample data loaded"
    );

    Ok(())
}

fn parse_args() -> anyhow::Result<Args> {
    let mut values = env::args().skip(1);
    let first = values.next();
    if matches!(first.as_deref(), Some("-h" | "--help")) {
        print_usage();
        std::process::exit(0);
    }

    let scenario = first
        .as_deref()
        .unwrap_or("full")
        .parse()
        .context("selecting a sample-data scenario")?;
    let mut week_start = current_week_start();

    while let Some(argument) = values.next() {
        match argument.as_str() {
            "--week-start" => {
                let raw = values
                    .next()
                    .context("--week-start needs a date in YYYY-MM-DD format")?;
                week_start = parse_date(&raw)?;
            }
            _ => bail!("unknown argument `{argument}`"),
        }
    }

    if week_start.weekday() != Weekday::Monday {
        bail!("--week-start must be a Monday");
    }

    let today = today_within(week_start, OffsetDateTime::now_utc().date());

    Ok(Args {
        scenario,
        week_start,
        today,
    })
}

fn today_within(week_start: Date, real_today: Date) -> Date {
    let offset = i64::from(real_today.weekday().number_days_from_monday());
    week_start + Duration::days(offset)
}

fn parse_date(value: &str) -> anyhow::Result<Date> {
    Date::parse(value, format_description!("[year]-[month]-[day]"))
        .with_context(|| format!("`{value}` is not a valid date"))
}

fn current_week_start() -> Date {
    let today = OffsetDateTime::now_utc().date();
    today - Duration::days(i64::from(today.weekday().number_days_from_monday()))
}

fn print_usage() {
    println!("Usage: sample-data [minimal|full] [--week-start YYYY-MM-DD]");
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::date;

    #[test]
    fn the_current_week_starts_on_a_monday() {
        assert_eq!(current_week_start().weekday(), Weekday::Monday);
    }

    #[test]
    fn dates_use_the_expected_format() {
        assert_eq!(parse_date("2026-08-24").unwrap(), date!(2026 - 08 - 24));
        assert!(parse_date("24-08-2026").is_err());
    }

    #[test]
    fn today_lands_on_the_matching_weekday_inside_the_chosen_week() {
        let week_start = date!(2026 - 08 - 24);
        assert_eq!(week_start.weekday(), Weekday::Monday);

        let today = today_within(week_start, date!(2026 - 09 - 03));
        assert_eq!(today, date!(2026 - 08 - 27));
        assert_eq!(today.weekday(), Weekday::Thursday);

        assert_eq!(today_within(week_start, week_start), week_start);
        assert!(today_within(week_start, date!(2030 - 01 - 06)) <= week_start + Duration::days(6));
    }
}
