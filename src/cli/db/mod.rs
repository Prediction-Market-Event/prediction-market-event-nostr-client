use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

use anyhow::{bail, Result};
use home::home_dir;
use nostr_sdk::{Keys, Url};
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::Row;
use sqlx::{Pool, Sqlite, SqlitePool};

use super::Context;

pub mod table_text_json;

pub const DATA_DIR: &str = ".prediction_market_event_cli";
pub fn get_data_dir() -> Result<PathBuf> {
    let Some(mut path_buf) = home_dir() else {
        bail!("failed to get home dir")
    };
    path_buf.extend([DATA_DIR]);

    fs::create_dir_all(&path_buf)?;

    Ok(path_buf)
}
pub async fn get_db() -> Result<Pool<Sqlite>> {
    let mut path = get_data_dir()?;
    path.extend(["sqlite.db"]);
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true);
    let db_pool = SqlitePool::connect_with(options).await?;
    initialize_db(&db_pool).await?;

    Ok(db_pool)
}

pub async fn initialize_db(db_pool: &Pool<Sqlite>) -> Result<()> {
    NostrSecretKey::init_table(db_pool).await?;
    NostrRelays::init_table(db_pool).await?;

    Ok(())
}

pub struct NostrSecretKey;
table_text_json::impl_table!(NostrSecretKey, "nostr_secret_key", String);

impl NostrSecretKey {
    pub async fn get_keys(context: &Context) -> Result<Keys> {
        let mut secret_key_option = Self::get(context, "").await?;
        if let None = secret_key_option {
            let keys = Keys::generate();
            let secret_key = keys.secret_key().to_secret_hex();
            Self::put(context, "", &secret_key).await?;
            secret_key_option = Some(secret_key);
        }
        let keys = Keys::parse(secret_key_option.unwrap())?;

        Ok(keys)
    }

    pub async fn set_keys(context: &Context, keys: Option<&Keys>) -> Result<()> {
        match keys {
            Some(keys) => {
                let secret_key_hex = keys.secret_key().to_secret_hex();
                Self::put(context, "", &secret_key_hex).await?;
            }
            None => {
                Self::delete(context, "").await?;
            }
        }

        Ok(())
    }
}

pub struct NostrRelays;
table_text_json::impl_table!(NostrRelays, "nostr_relays", ());

impl NostrRelays {
    pub async fn add_url(context: &Context, url: Url) -> Result<()> {
        let key = url.to_string();
        Self::put(context, key, &()).await?;

        Ok(())
    }

    pub async fn remove_url(context: &Context, url: Url) -> Result<()> {
        let key = url.to_string();
        Self::delete(context, key).await?;

        Ok(())
    }

    pub async fn remove_all_urls(context: &Context) -> Result<()> {
        Self::delete_all(context).await?;

        Ok(())
    }

    pub async fn get_all_urls(context: &Context) -> Result<Vec<Url>> {
        let mut h = Vec::new();
        for (url_string, _) in Self::get_all(context).await?.into_iter() {
            let url = Url::from_str(&url_string)?;
            h.push(url);
        }

        Ok(h)
    }
}
