use std::io::{self, Write};

use anyhow::{bail, Result};
use chrono::{DateTime, NaiveDateTime, Utc};
use prediction_market_event::{
    information::{Information, None, V1},
    Event, Outcome, PayoutUnit,
};

pub fn information_creator_prompt(
    information_type: &str,
    outcome_count: Outcome,
) -> Result<Information> {
    let information = match information_type.to_ascii_lowercase().as_str() {
        None::ID => Information::None,
        V1::ID => {
            let title = read_line("Title");
            let description = read_line("Description");

            let mut outcome_titles = vec![String::new(); outcome_count.into()];
            for (outcome, outcome_title) in outcome_titles.iter_mut().enumerate() {
                *outcome_title = read_line(&format!("Outcome Title {outcome}"));
            }

            let datetime_string =
                read_line("Expected Payout Date Time UTC (format: `2023-10-01T12:00:00Z`)");
            let datetime: DateTime<Utc> = DateTime::from_naive_utc_and_offset(
                NaiveDateTime::parse_from_str(&datetime_string, "%Y-%m-%dT%H:%M:%SZ")?,
                Utc,
            );
            let expected_payout_unix_seconds: u64 = datetime.timestamp().try_into()?;

            Information::V1(V1 {
                title,
                description,
                outcome_titles,
                expected_payout_unix_seconds,
            })
        }

        _ => bail!("unsupported information type"),
    };

    Ok(information)
}

pub fn event_payout_units_per_outcome_creator_prompt(event: &Event) -> Result<Vec<PayoutUnit>> {
    println!(
        "{} units available to distribute between {} outcomes.",
        event.units_to_payout, event.outcome_count
    );

    let mut outcome_titles = Vec::new();
    match &event.information {
        Information::None => {
            for i in 0..event.outcome_count {
                outcome_titles.push(format!("Outcome {i}"));
            }
        }
        Information::V1(v1) => outcome_titles = v1.outcome_titles.to_owned(),
    }

    let mut units_per_outcome = Vec::new();
    for outcome_title in outcome_titles {
        let prompt = format!("Payout to {outcome_title}");
        let outcome_payout: PayoutUnit = read_line(&prompt).parse()?;
        units_per_outcome.push(outcome_payout);
    }

    if &read_line("Review your entry and enter 'y' to confirm your entry") != "y" {
        bail!("units per outcome entry canceled")
    }

    Ok(units_per_outcome)
}

fn read_line(prompt: &str) -> String {
    print!("{prompt} >> ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_owned()
}
