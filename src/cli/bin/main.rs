use prediction_market_event_nostr_client::cli::parse_and_run;

#[tokio::main]
async fn main() {
    match parse_and_run().await {
        Ok(v) => {
            let json_pretty = serde_json::to_string_pretty(&v).expect("failed to cli value");
            println!("{json_pretty}")
        }
        Err(e) => {
            println!("{e}")
        }
    }
}