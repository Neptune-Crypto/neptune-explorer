use axum::http::HeaderValue;
use futures::future::join_all;
use neptune_explorer::path::ExplorerPath;
use rand::rng;
use rand::Rng;
use reqwest::Client;
use tokio::time::Instant;

///
/// Run with: `> cargo run --bin high_rate_attack --features "attacks"`
#[tokio::main]
async fn main() {
    let client = Client::new();
    let root_url = "http://127.0.0.1:3000/";
    let num_requests = 2000; // adjust as needed
    let concurrency = 20; // parallel tasks

    let start = Instant::now();
    let mut rng = rng();
    let futures = (0..num_requests).map(|_| {
        let path = rng.random::<ExplorerPath>().to_string();
        let client = &client;
        async move {
            let _ = client
                .get([root_url, &path].concat())
                .header("X-Real-IP-Override", HeaderValue::from_static("1.2.3.4"))
                .send()
                .await;
        }
    });

    join_all(futures).await;

    let elapsed = start.elapsed();
    println!("Sent {num_requests} requests in {elapsed:.2?} (concurrency {concurrency})");
}
