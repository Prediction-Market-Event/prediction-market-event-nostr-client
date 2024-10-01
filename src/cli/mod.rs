use anyhow::Result;
use clap::Parser;
use db::get_db;
use parser::Cli;
use sqlx::{Pool, Sqlite};

use crate::{client::Signer, Client};

pub mod db;
pub mod parser;
pub mod stdin_prompts;

pub struct Context {
    pub db_pool: Pool<Sqlite>,
}
impl Context {
    pub async fn get() -> Result<Context> {
        let context = Self {
            db_pool: get_db().await?,
        };

        Ok(context)
    }

    pub async fn client(&self) -> Result<Client<Signer>> {
        let relays = db::NostrRelays::get_all_urls(self).await?;
        let keys = db::NostrSecretKey::get_keys(self).await?;

        Client::new_initialized_client_signer(relays, keys).await
    }
}

pub async fn parse_and_handle() -> Result<serde_json::Value> {
    let context = Context::get().await?;
    let cli = Cli::parse();

    cli.handle(&context).await
}
