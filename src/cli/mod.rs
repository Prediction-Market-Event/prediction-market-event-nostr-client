use anyhow::Result;
use clap::Parser;
use nostr_sdk::{Keys, Url};
use parser::Cli;

use crate::Client;

pub mod parser;

pub async fn parse_and_run() -> Result<serde_json::Value>{
    let cli = Cli::parse();
    cli.handle().await
}

pub async fn client() -> Result<Client> {
    let keys = Keys::generate();
    let relays = vec![Url::parse("ws://127.0.0.1:8080").unwrap()];

    Client::new_initialized_client(keys, relays).await
}