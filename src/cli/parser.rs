use std::str::FromStr;

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use nostr_sdk::{Keys, PublicKey, Url};
use prediction_market_event::{
    information::Information, nostr_event_types::NewEvent, Event, Outcome, PayoutUnit,
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
    Relay {
        #[command(subcommand)]
        relay_commands: RelayCommands,
    },
    Key {
        #[command(subcommand)]
        key_commands: KeyCommand,
    },
    Publish {
        #[command(subcommand)]
        publish_commands: PublishCommands,
    },
    Query {
        #[command(subcommand)]
        query_commands: QueryCommands,
    },
}

#[derive(Subcommand)]
pub enum RelayCommands {
    Add { url: String },
    Remove { url: String },
    RemoveAll,
    GetAll,
}

#[derive(Subcommand)]
pub enum KeyCommand {
    Public,
    Secret,
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

#[derive(Subcommand)]
pub enum QueryCommands {
    Event { author: Option<String> },
}

impl Cli {
    pub async fn handle(self, context: &Context) -> Result<serde_json::Value> {
        let Some(command) = self.command else {
            bail!("No command provided. Use --help for more information.");
        };

        let json = match command {
            Commands::Relay { relay_commands } => match relay_commands {
                RelayCommands::Add { url } => {
                    let url = Url::from_str(&url)?;
                    db::NostrRelays::add_url(context, url).await?;

                    json!(true)
                }
                RelayCommands::Remove { url } => {
                    let url = Url::from_str(&url)?;
                    db::NostrRelays::remove_url(context, url).await?;

                    json!(true)
                }
                RelayCommands::RemoveAll => {
                    db::NostrRelays::remove_all_urls(context).await?;

                    json!(true)
                }
                RelayCommands::GetAll => {
                    let all_urls = db::NostrRelays::get_all_urls(context).await?;

                    json!(all_urls)
                }
            },
            Commands::Key {
                key_commands: keys_commands,
            } => match keys_commands {
                KeyCommand::Public => {
                    let keys = db::NostrSecretKey::get_keys(context).await?;

                    json!(keys.public_key.to_hex())
                }
                KeyCommand::Secret => {
                    let keys = db::NostrSecretKey::get_keys(context).await?;

                    json!(keys.secret_key().to_secret_hex())
                }
                KeyCommand::Set { secret_key } => {
                    let keys = Keys::parse(secret_key)?;
                    db::NostrSecretKey::set_keys(context, Some(&keys)).await?;

                    json!(true)
                }
                KeyCommand::Delete => {
                    db::NostrSecretKey::set_keys(context, None).await?;

                    json!(true)
                }
            },
            Commands::Publish { publish_commands } => match publish_commands {
                PublishCommands::NewEvent {
                    outcome_count,
                    units_to_payout,
                    information_type,
                } => {
                    let information =
                        information_stdin_prompts::prompt(&information_type, outcome_count)?;
                    let event =
                        Event::new_with_random_nonce(outcome_count, units_to_payout, information);
                    event.validate(Information::ALL_VARIANT_IDS)?;

                    let success = context.client().await?.publish::<NewEvent>(&event).await?;

                    json!(success)
                }
            },

            Commands::Query { query_commands } => match query_commands {
                QueryCommands::Event { author } => {
                    let author_parsed = if let Some(author_string) = author {
                        Some(PublicKey::parse(author_string)?)
                    } else {
                        None
                    };

                    let events = context
                        .client()
                        .await?
                        .get::<NewEvent>(
                            |mut f| {
                                if let Some(pk) = author_parsed {
                                    f = f.author(pk);
                                }

                                f
                            },
                            None,
                        )
                        .await?;

                    json!(events)
                }
            },
        };

        Ok(json)
    }
}
