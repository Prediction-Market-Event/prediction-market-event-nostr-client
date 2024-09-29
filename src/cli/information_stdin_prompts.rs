use std::io::{self, Write};

use anyhow::{bail, Result};
use prediction_market_event::{
    information::{Information, None, V1},
    Outcome,
};

pub fn prompt(information_type: &str, outcome_count: Outcome) -> Result<Information> {
    let information = match information_type.to_ascii_lowercase().as_str() {
        None::ID => Information::None,
        V1::ID => {
            let title = read_line("Title");
            let description = read_line("Description");

            let mut outcome_titles = vec![String::new(); outcome_count.into()];
            for (outcome, outcome_title) in outcome_titles.iter_mut().enumerate() {
                *outcome_title = read_line(&format!("Outcome Title {outcome}"));
            }

            Information::V1(V1 {
                title,
                description,
                outcome_titles,
                expected_payout_unix_seconds: 100,
            })
        }

        _ => bail!("unsupported information type"),
    };

    Ok(information)
}

fn read_line(prompt: &str) -> String {
    print!("{prompt} >> ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_owned()
}
