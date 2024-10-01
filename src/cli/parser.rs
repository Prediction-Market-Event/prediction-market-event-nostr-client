use std::{collections::HashSet, str::FromStr};

use anyhow::{bail, Error, Result};
use clap::{Parser, Subcommand};
use nostr_sdk::{Filter, Keys, PublicKey, Timestamp, ToBech32, Url};
use prediction_market_event::{
    information::Information,
    nostr_event_types::{
        EventPayoutAttestation, FutureEventPayoutAttestationPledge, NewEvent, NostrEventUtils,
    },
    Event, EventHashHex, EventPayout, Outcome, PayoutUnit,
};
use serde_json::json;

use crate::cli::{db, stdin_prompts, Context};

#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    Key {
        #[command(subcommand)]
        key_commands: KeyCommand,
    },
    Relay {
        #[command(subcommand)]
        relay_commands: RelayCommands,
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
pub enum KeyCommand {
    Public,
    Secret,
    Set { secret_key: String },
    Delete,
}

#[derive(Subcommand)]
pub enum RelayCommands {
    Add { url: String },
    Remove { url: String },
    RemoveAll,
    ListAll,
    AddRecommendedRelayList,
}

#[derive(Subcommand)]
pub enum PublishCommands {
    NewEvent {
        outcome_count: Outcome,
        units_to_payout: PayoutUnit,
        information_type: String,
    },
    FutureEventPayoutAttestationPledge {
        event_hash_hex: EventHashHex,
    },
    EventPayoutAttestation {
        event_hash_hex: EventHashHex,
    },
}

#[derive(Subcommand)]
pub enum QueryCommands {
    Custom {
        #[arg(short, long)]
        limit: Option<usize>,
        #[arg(short, long)]
        author: Option<PublicKey>,
        #[arg(short, long)]
        since: Option<Timestamp>,
        #[arg(short, long)]
        until: Option<Timestamp>,
        #[arg(short, long)]
        event_hash_hex: Option<EventHashHex>,

        #[command(subcommand)]
        query_custom_commands: QueryCustomCommands,
    },
    MyCreatedEvents,
    EventsPendingYourAttestation,
}

#[derive(Subcommand)]
pub enum QueryCustomCommands {
    NewEvent,
    FutureEventPayoutAttestationPledge,
    EventPayoutAttestation,
}

impl Cli {
    pub async fn handle(self, context: &Context) -> Result<serde_json::Value> {
        let Some(command) = self.command else {
            bail!("No command provided. Use --help for more information.");
        };

        let json = match command {
            Commands::Key {
                key_commands: keys_commands,
            } => match keys_commands {
                KeyCommand::Public => {
                    let keys = db::NostrSecretKey::get_keys(context).await?;

                    json!(keys.public_key.to_bech32()?)
                }
                KeyCommand::Secret => {
                    let keys = db::NostrSecretKey::get_keys(context).await?;

                    json!(keys.secret_key().to_bech32()?)
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
                RelayCommands::ListAll => {
                    let all_urls = db::NostrRelays::get_all_urls(context).await?;

                    json!(all_urls)
                }
                RelayCommands::AddRecommendedRelayList => {
                    for relay in RECOMMENDED_RELAY_LIST {
                        db::NostrRelays::add_url(context, Url::from_str(&relay)?).await?;
                    }

                    json!(true)
                }
            },

            Commands::Publish { publish_commands } => match publish_commands {
                PublishCommands::NewEvent {
                    outcome_count,
                    units_to_payout,
                    information_type,
                } => {
                    let information = stdin_prompts::information_creator_prompt(
                        &information_type,
                        outcome_count,
                    )?;
                    let event =
                        Event::new_with_random_nonce(outcome_count, units_to_payout, information);
                    event.validate(Information::ALL_VARIANT_IDS)?;
                    let event_hash_hex = event.hash_hex()?;

                    let success = context.client().await?.publish::<NewEvent>(&event).await?;

                    json!({
                        "hash_hex": event_hash_hex,
                        "relays": success,
                    })
                }
                PublishCommands::FutureEventPayoutAttestationPledge { event_hash_hex } => {
                    let success = context
                        .client()
                        .await?
                        .publish::<FutureEventPayoutAttestationPledge>(&event_hash_hex)
                        .await?;

                    json!({
                        "relays": success,
                    })
                }
                PublishCommands::EventPayoutAttestation { event_hash_hex } => {
                    let res = context
                        .client()
                        .await?
                        .get::<NewEvent>(|f| vec![f.hashtag(event_hash_hex.0.to_owned())], None)
                        .await?;
                    let event = res
                        .get(0)
                        .map(|(_, e)| e)
                        .ok_or(Error::msg("could not get event with hash hex"))?;
                    let units_per_outcome =
                        stdin_prompts::event_payout_units_per_outcome_creator_prompt(event)?;
                    let event_payout = EventPayout {
                        event_hash_hex,
                        units_per_outcome,
                    };
                    event_payout.validate(event)?;

                    let success = context
                        .client()
                        .await?
                        .publish::<EventPayoutAttestation>(&event_payout)
                        .await?;

                    json!({
                        "relays": success,
                    })
                }
            },

            Commands::Query { query_commands } => match query_commands {
                QueryCommands::Custom {
                    author,
                    limit,
                    since,
                    until,
                    event_hash_hex,
                    query_custom_commands,
                } => {
                    let filter_fn = |mut f: Filter| {
                        if let Some(pk) = author {
                            f = f.author(pk);
                        }
                        if let Some(l) = limit {
                            f = f.limit(l);
                        }
                        if let Some(s) = since {
                            f = f.since(s);
                        }
                        if let Some(u) = until {
                            f = f.until(u);
                        }
                        if let Some(e) = event_hash_hex {
                            f = f.hashtag(e.0);
                        }

                        vec![f]
                    };

                    match query_custom_commands {
                        QueryCustomCommands::NewEvent => {
                            let res = context
                                .client()
                                .await?
                                .get::<NewEvent>(filter_fn, None)
                                .await?;

                            new_event_json(&res)
                        }
                        QueryCustomCommands::FutureEventPayoutAttestationPledge => {
                            let res = context
                                .client()
                                .await?
                                .get::<FutureEventPayoutAttestationPledge>(filter_fn, None)
                                .await?;

                            future_event_payout_attestation_pledge_json(&res)
                        }
                        QueryCustomCommands::EventPayoutAttestation => {
                            let res = context
                                .client()
                                .await?
                                .get::<EventPayoutAttestation>(filter_fn, None)
                                .await?;

                            event_payout_attestation_json(&res)
                        }
                    }
                }
                QueryCommands::MyCreatedEvents => {
                    let author = db::NostrSecretKey::get_keys(context).await?.public_key;

                    let res = context
                        .client()
                        .await?
                        .get::<NewEvent>(|f| vec![f.author(author).limit(100)], None)
                        .await?;

                    new_event_json(&res)
                }
                QueryCommands::EventsPendingYourAttestation => {
                    let author = db::NostrSecretKey::get_keys(context).await?.public_key;

                    let events_with_future_event_payout_attestation_pledge: HashSet<_> = context
                        .client()
                        .await?
                        .get::<FutureEventPayoutAttestationPledge>(|f| vec![f.author(author)], None)
                        .await?
                        .into_iter()
                        .map(|(_, (pk, event))| {
                            assert_eq!(author.to_hex(), pk.0);
                            event
                        })
                        .collect();

                    let events_with_event_payout_attestation: HashSet<_> = context
                        .client()
                        .await?
                        .get::<EventPayoutAttestation>(
                            |f| {
                                events_with_future_event_payout_attestation_pledge
                                    .iter()
                                    .map(|event_hash_hex| {
                                        f.clone().author(author).hashtag(&event_hash_hex.0)
                                    })
                                    .collect()
                            },
                            None,
                        )
                        .await?
                        .into_iter()
                        .map(|(_, (pk, event_payout))| {
                            assert_eq!(author.to_hex(), pk.0);
                            event_payout.event_hash_hex
                        })
                        .collect();

                    let events_pending_attestation: Vec<_> =
                        events_with_future_event_payout_attestation_pledge
                            .difference(&events_with_event_payout_attestation)
                            .collect();
                    let res: Vec<_> = context
                        .client()
                        .await?
                        .get::<NewEvent>(
                            |f| {
                                events_pending_attestation
                                    .iter()
                                    .map(|event_hash_hex| f.clone().hashtag(&event_hash_hex.0))
                                    .collect()
                            },
                            None,
                        )
                        .await?;

                    new_event_json(&res)
                }
            },
        };

        Ok(json)
    }
}

fn new_event_json(
    res: &Vec<(
        nostr_sdk::Event,
        <NewEvent as NostrEventUtils>::InterpretResult,
    )>,
) -> serde_json::Value {
    let events: Vec<_> = res
        .iter()
        .map(|(_, p)| {
            let event_hash_hex = p.hash_hex().expect("failed to get hash hex of event");
            json!({"event_hash_hex": event_hash_hex, "event": p})
        })
        .collect();

    json!(events)
}

fn future_event_payout_attestation_pledge_json(
    res: &Vec<(
        nostr_sdk::Event,
        <FutureEventPayoutAttestationPledge as NostrEventUtils>::InterpretResult,
    )>,
) -> serde_json::Value {
    let events: Vec<_> = res
        .iter()
        .map(|(_, p)| json!({"future_attesation_pledge_maker": p.0, "event_hash_hex": p.1}))
        .collect();

    json!(events)
}

fn event_payout_attestation_json(
    res: &Vec<(
        nostr_sdk::Event,
        <EventPayoutAttestation as NostrEventUtils>::InterpretResult,
    )>,
) -> serde_json::Value {
    let events: Vec<_> = res
        .iter()
        .map(|(_, p)| json!({"attestor": p.0, "event_payout": p.1}))
        .collect();

    json!(events)
}

const RECOMMENDED_RELAY_LIST: &[&str] = &[
    "wss://btc.klendazu.com",
    "wss://nostr.yael.at",
    "wss://nostr.oxtr.dev",
    "wss://relay.lexingtonbitcoin.org",
    "wss://nos.lol",
    "wss://nostr.bitcoiner.social",
    "wss://relay.primal.net",
    "wss://nostrrelay.com",
];
