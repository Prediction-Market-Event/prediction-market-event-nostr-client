use anyhow::Result;
use clap::Parser;
use db::get_db;
use nostr_sdk::Url;
use parser::Cli;
use rocksdb::DB;

use crate::Client;

pub mod db;
pub mod parser;

pub struct Context {
    pub db: DB,
}
impl Context {
    pub fn create() -> Result<Context> {
        let context = Self { db: get_db()? };

        Ok(context)
    }

    pub async fn client(&self) -> Result<Client> {
        let keys = db::NostrSecretKey::get_keys(self)?;
        let relays = vec![Url::parse("ws://127.0.0.1:8080").unwrap()];

        Client::new_initialized_client(keys, relays).await
    }
}

pub async fn parse_and_run() -> Result<serde_json::Value> {
    let context = Context::create()?;

    let cli = Cli::parse();
    cli.handle(&context).await
}
