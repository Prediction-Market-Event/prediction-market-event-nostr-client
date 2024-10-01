use std::{collections::HashSet, marker::PhantomData, time::Duration};

use anyhow::Result;
use nostr_sdk::{Filter, Keys, Url};
use prediction_market_event::nostr_event_types::NostrEventUtils;

pub struct Client<State = QueryOnly> {
    keys: Option<Keys>,
    nostr_client: nostr_sdk::Client,

    state: PhantomData<State>,
}
pub struct QueryOnly;
pub struct Signer;

impl Client {
    pub async fn new_initialized_client_query_only(relays: Vec<Url>) -> Result<Client<QueryOnly>> {
        let nostr_client = nostr_sdk::Client::default();
        for relay in relays {
            nostr_client.add_relay(relay).await?;
        }
        nostr_client.connect().await;

        Ok(Client {
            keys: None,
            nostr_client,
            state: PhantomData::<QueryOnly>,
        })
    }
    pub async fn new_initialized_client_signer(
        relays: Vec<Url>,
        keys: Keys,
    ) -> Result<Client<Signer>> {
        let client_query_only = Self::new_initialized_client_query_only(relays).await?;

        Ok(Client {
            keys: Some(keys),
            nostr_client: client_query_only.nostr_client,
            state: PhantomData::<Signer>,
        })
    }
}
impl<State> Client<State> {
    pub async fn get<PredictionMarketEventNostrEventType>(
        &self,
        filter_fn: impl FnOnce(Filter) -> Vec<Filter>,
        request_timeout: Option<Duration>,
    ) -> Result<
        Vec<(
            nostr_sdk::Event,
            PredictionMarketEventNostrEventType::InterpretResult,
        )>,
    >
    where
        PredictionMarketEventNostrEventType: NostrEventUtils,
    {
        let filters = filter_fn(PredictionMarketEventNostrEventType::filter());

        let nostr_event_vec = self
            .nostr_client
            .get_events_of(filters, nostr_sdk::EventSource::both(request_timeout))
            .await?;

        let mut interpret_vec = Vec::new();
        for nostr_event in nostr_event_vec {
            if let Ok(interpret_result) =
                PredictionMarketEventNostrEventType::interpret_nostr_event(&nostr_event)
            {
                interpret_vec.push((nostr_event, interpret_result));
            }
        }

        Ok(interpret_vec)
    }
}

impl Client<Signer> {
    pub async fn publish<PredictionMarketEventNostrEventType>(
        &self,
        params: &PredictionMarketEventNostrEventType::CreateParameter,
    ) -> Result<HashSet<Url>>
    where
        PredictionMarketEventNostrEventType: NostrEventUtils,
    {
        let nostr_event = PredictionMarketEventNostrEventType::create_nostr_event_builder(params)?
            .to_event(self.keys.as_ref().unwrap())?;
        let output = self.nostr_client.send_event(nostr_event).await?;

        Ok(output.success)
    }
}
