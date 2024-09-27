use anyhow::{bail, Result};
use prediction_market_event::{
    information::{Information, None, V1},
    Outcome,
};

pub fn prompt(information_type: &str, outcome_count: Outcome) -> Result<Information> {
    let information = match information_type {
        None::ID => Information::None,
        V1::ID => Information::V1(V1 {
            title: "test".to_string(),
            description: "this is a test".to_string(),
            outcome_titles: vec!["".to_string(); outcome_count as usize],
            expected_payout_unix_seconds: 100,
        }),

        _ => bail!("unsupported information type"),
    };

    Ok(information)
}
