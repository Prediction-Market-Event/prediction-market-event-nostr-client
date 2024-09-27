use std::path::PathBuf;

use anyhow::{bail, Result};
use home::home_dir;
use nostr_sdk::Keys;
use rocksdb::{Options, DB};
use serde::{de::DeserializeOwned, Serialize};

use super::Context;

pub const DATA_DIR: &str = ".prediction_market_event_cli";
pub fn get_data_dir() -> Result<PathBuf> {
    let Some(mut path_buf) = home_dir() else {
        bail!("failed to get home dir")
    };
    path_buf.extend([DATA_DIR]);

    Ok(path_buf)
}
pub fn get_db() -> Result<DB> {
    let mut opts = Options::default();
    opts.create_if_missing(true);
    let mut data_dir = get_data_dir()?;
    data_dir.extend(["db"]);

    let db = DB::open(&opts, &data_dir)?;

    Ok(db)
}

pub trait DBKeyValue {
    const KEY: &'static str;
    type Value: Serialize + DeserializeOwned;
}

pub struct NostrSecretKey;
impl DBKeyValue for NostrSecretKey {
    const KEY: &'static str = "nostr_secret_key";
    type Value = String;
}
impl NostrSecretKey {
    pub fn get_keys(context: &Context) -> Result<Keys> {
        let mut secret_key_option = context.get_kv_in_db::<Self>()?;
        if let None = secret_key_option {
            let keys = Keys::generate();
            let secret_key = keys.secret_key().to_secret_hex();
            context.put_kv_in_db::<Self>(&secret_key)?;
            secret_key_option = Some(secret_key);
        }
        let keys = Keys::parse(secret_key_option.unwrap())?;

        Ok(keys)
    }

    pub fn set_keys(context: &Context, keys: Option<&Keys>) -> Result<()> {
        match keys {
            Some(keys) => {
                let secret_key_hex = keys.secret_key().to_secret_hex();
                context.put_kv_in_db::<Self>(&secret_key_hex)?;
            }
            None => {
                context.delete_kv_in_db::<Self>()?;
            }
        }

        Ok(())
    }
}

impl Context {
    pub fn put_kv_in_db<KV: DBKeyValue>(&self, value: &KV::Value) -> Result<()> {
        let value_json = serde_json::to_string(value)?;
        self.db.put(KV::KEY, &value_json)?;

        Ok(())
    }

    pub fn get_kv_in_db<KV: DBKeyValue>(&self) -> Result<Option<KV::Value>> {
        let value_json = self.db.get(KV::KEY)?;
        let Some(value_json) = value_json else {
            return Ok(None);
        };
        let value = serde_json::from_slice(&value_json)?;

        Ok(Some(value))
    }

    pub fn delete_kv_in_db<KV: DBKeyValue>(&self) -> Result<()> {
        self.db.delete(KV::KEY)?;

        Ok(())
    }
}
