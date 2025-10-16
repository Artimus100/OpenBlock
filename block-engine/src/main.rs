use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Duration};
use std::collections::HashMap;
use sha2::{Digest, Sha256};
use reqwest::Client;

// --- Bundle Data ---
#[derive(Debug, Serialize, Deserialize, Clone)]
struct Bundle {
    id: String,
    transactions: Vec<String>,
    tip: u64,
    searcher_pubkey: String,
    timestamp: u64,
}

// --- Ordered Block ---
#[derive(Debug, Serialize, Deserialize)]
struct OrderedBlock {
    window_id: u64,
    ordered_bundles: Vec<Bundle>,
    ordered_hash: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🧠 Block Engine: Listening for bundles...");

    let client = Client::new();
    let redis_client = redis::Client::open("redis://127.0.0.1/")?;
    let mut con = redis_client.get_async_connection().await?;

    loop {
        let window_id = (chrono::Utc::now().timestamp_millis() / 200) as u64;
        let key = format!("bundle_window:{}", window_id);

        let bundles_json: Vec<String> = con.lrange(&key, 0, -1).await.unwrap_or_default();
        if bundles_json.is_empty() {
            sleep(Duration::from_millis(100)).await;
            continue;
        }

        let mut bundles: Vec<Bundle> = bundles_json
            .iter()
            .filter_map(|b| serde_json::from_str(b).ok())
            .collect();

        // Deterministic ordering by tip, then hash of bundle ID (for tie-breaking)
        bundles.sort_by_key(|b| (b.tip, hash_str(&b.id)));

        // Create deterministic ordered hash
        let mut hasher = Sha256::new();
        for b in &bundles {
            hasher.update(b.id.as_bytes());
            hasher.update(b.tip.to_le_bytes());
        }
        let ordered_hash = format!("{:x}", hasher.finalize());

        let block = OrderedBlock {
            window_id,
            ordered_bundles: bundles.clone(),
            ordered_hash,
        };

        println!(
            "✅ Built block for window {} with {} bundles → hash: {}",
            window_id,
            block.ordered_bundles.len(),
            &block.ordered_hash[..16]
        );

        // Optional: send to mock validator
        let _ = client.post("http://localhost:4000/submit_block")
            .json(&block)
            .send()
            .await;

        // Clean up Redis key after processing
        let _: () = con.del(&key).await?;
        sleep(Duration::from_millis(200)).await;
    }
}

// --- helper: hash a string deterministically ---
fn hash_str(input: &str) -> u64 {
    use std::hash::{Hasher, Hash};
    use std::collections::hash_map::DefaultHasher;
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    hasher.finish()
}
