use futures::future::join_all;
use reqwest::Client;
use tokio::time::Instant;

#[tokio::main]
async fn main() {
    let client = Client::new();
    let url = "http://127.0.0.1:3000/your_endpoint";
    let num_requests = 200; // adjust as needed
    let concurrency = 20; // parallel tasks

    let start = Instant::now();
    let futures = (0..num_requests).map(|_| {
        let client = &client;
        async move {
            let _ = client.get(url).send().await;
        }
    });

    join_all(futures).await;

    let elapsed = start.elapsed();
    println!(
        "Sent {} requests in {:.2?} (concurrency {})",
        num_requests, elapsed, concurrency
    );
}
