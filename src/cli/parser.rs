use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use nostr_sdk::Keys;
use prediction_market_event::{
    nostr_event_types::NewEvent, Event, Outcome, PayoutUnit,
};
use serde_json::json;

use crate::cli::{db, information_stdin_prompts, Context};

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
    Publish {
        #[command(subcommand)]
        publish_commands: PublishCommands,
    },
    Get,
}

#[derive(Subcommand)]
pub enum KeysCommand {
    PublicHex,
    SecretHex,
    Set { secret_key: String },
    Delete,
}

#[derive(Subcommand)]
pub enum PublishCommands {
    NewEvent {
        outcome_count: Outcome,
        units_to_payout: PayoutUnit,
        information_type: String,
    },
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
            Commands::Publish { publish_commands } => match publish_commands {
                PublishCommands::NewEvent { outcome_count, units_to_payout, information_type } => {
                    let information = information_stdin_prompts::prompt(&information_type, outcome_count)?;
                    let event = Event::new_with_random_nonce(outcome_count, units_to_payout, information);

                    let success = context
                        .client()
                        .await?
                        .publish::<NewEvent>(&event)
                        .await?;

                    json!(success)
                }
            },

            Commands::Get => {
                let events = context.client().await?.get::<NewEvent>(|f| f, None).await?;

                json!(events)
            }
        };

        Ok(json)
    }
}
