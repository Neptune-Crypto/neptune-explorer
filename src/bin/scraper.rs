use env_logger;
use log::LevelFilter;
use log::{error, info, warn};
use rand::seq::IteratorRandom;
use regex::Regex;
use reqwest::Client;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::{signal, time};
use url::Url;

/// Scrape the explorer website when running locally.
///
/// This program maintains a dictionary of URLs, which is initially populated
/// with 'http://localhost:3000'. It fetches a random URL from the dictionary in
/// each iteration, logs positive messages if successful, extracts new URLs from
/// the response body to add to the dictionary, logs warnings or errors for
/// request failures or timeouts, sleeps for a bit, and continues until Ctrl-C
/// is pressed.
///
/// Run with:
///  `> cargo run --bin scraper`
#[tokio::main]
async fn main() {
    // Initialize logger
    env_logger::builder().filter_level(LevelFilter::Info).init();

    let client = Client::builder()
        .timeout(Duration::from_millis(300))
        .build()
        .expect("Failed to build HTTP client");

    let root_url = "http://localhost:3000".to_string();
    let urls = Arc::new(Mutex::new(HashSet::from([root_url.clone()])));

    let href_regex = Regex::new(r#"<a\s+(?:[^>]*?\s+)?href=['\"](.*?)['\"]"#).unwrap();

    info!("Starting fetch loop. Press Ctrl-C to stop.");

    let urls_clone = Arc::clone(&urls);
    let fetch_loop = async move {
        loop {
            // Pick a random URL safely
            let url_opt = {
                let urls_guard = urls_clone.lock().unwrap();
                urls_guard.iter().choose(&mut rand::rng()).cloned()
            };

            if let Some(url) = url_opt {
                match client.get(&url).send().await {
                    Ok(resp) => {
                        if resp.status().is_success() {
                            match resp.text().await {
                                Ok(text) => {
                                    info!("Success fetching {}", url);
                                    let mut urls_guard = urls_clone.lock().unwrap();
                                    for cap in href_regex.captures_iter(&text) {
                                        let href = &cap[1];
                                        if let Ok(parsed_url) =
                                            Url::parse(&[&root_url.clone(), href].concat())
                                        {
                                            let normalized = parsed_url.as_str();
                                            if urls_guard.insert(normalized.to_owned()) {
                                                info!(
                                                    "Added new URL to dictionary: {}",
                                                    normalized
                                                );
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    warn!("Failed to read response body from {}: {}", url, e);
                                }
                            }
                        } else {
                            warn!("Non-success status {} from {}", resp.status(), url);
                        }
                    }
                    Err(err) => {
                        if err.is_timeout() {
                            warn!("Timeout fetching {}", url);
                        } else {
                            error!("Error fetching {}: {}", url, err);
                        }
                    }
                }
            } else {
                warn!("URL dictionary is empty, no URL to fetch");
            }

            time::sleep(Duration::from_millis(500)).await;
        }
    };

    tokio::select! {
        _ = fetch_loop => {}, // This runs indefinitely unless stopped
        _ = signal::ctrl_c() => {
            info!("Ctrl-C received, stopping...");
        }
    }
}
