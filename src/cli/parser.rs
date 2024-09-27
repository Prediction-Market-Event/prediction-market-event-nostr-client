use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use nostr_sdk::Keys;
use prediction_market_event::{information::Information, nostr_event_types::NewEvent, Event};
use serde_json::json;

use crate::cli::db;

use super::Context;

#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    Keys {
        #[command(subcommand)]
        keys_commands: KeysCommand,
    },
    Publish,
    Get,
}

#[derive(Subcommand)]
pub enum KeysCommand {
    PublicHex,
    SecretHex,
    Set { secret_key: String },
    Delete,
}

impl Cli {
    pub async fn handle(self, context: &Context) -> Result<serde_json::Value> {
        let Some(command) = self.command else {
            bail!("No command provided. Use --help for more information.");
        };

        let json = match command {
            Commands::Keys { keys_commands } => match keys_commands {
                KeysCommand::PublicHex => {
                    let keys = db::NostrSecretKey::get_keys(context)?;

                    json!(keys.public_key.to_hex())
                }
                KeysCommand::SecretHex => {
                    let keys = db::NostrSecretKey::get_keys(context)?;

                    json!(keys.secret_key().to_secret_hex())
                }
                KeysCommand::Set { secret_key } => {
                    let keys = Keys::parse(secret_key)?;
                    db::NostrSecretKey::set_keys(context, Some(&keys))?;

                    json!(true)
                }
                KeysCommand::Delete => {
                    db::NostrSecretKey::set_keys(context, None)?;

                    json!(true)
                }
            },
            Commands::Publish => {
                let client = context.client().await?;
                let success = client
                    .publish::<NewEvent>(&Event::new_with_random_nonce(3, 5, Information::None))
                    .await?;

                json!(success)
            }
            Commands::Get => {
                let client = context.client().await?;
                let events = client.get::<NewEvent>(|f| f, None).await?;

                json!(events)
            }
        };

        Ok(json)
    }
}
