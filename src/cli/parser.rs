use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use prediction_market_event::{information::Information, nostr_event_types::NewEvent, Event};

use crate::cli::client;

macro_rules! json {
    ($v:expr) => {
        return Ok(serde_json::to_value($v)?)
    };
}

#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    Publish,
    Get
}

impl Cli {
    pub async fn handle(self) -> Result<serde_json::Value> {
        let Some(command) = self.command else {
            bail!("No command provided. Use --help for more information.");
        };

        match command {
            Commands::Publish => {
                let client = client().await?;
                let success = client
                    .publish::<NewEvent>(&Event::new_with_random_nonce(3, 5, Information::None))
                    .await?;

                json!(success)
            }
            Commands::Get => {
                let client = client().await?;
                let events = client.get::<NewEvent>(|f| f, None).await?;

                json!(events)
            }
        }
    }
}
