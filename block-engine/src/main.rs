use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Duration};
use std::collections::HashMap;
use sha2::{Digest, Sha256};
use reqwest::Client;
use tracing::{info, warn, debug, Level};
use tracing_subscriber;

// Import our auction modules
mod auction;
mod bundle;
mod simulator;
use auction::{simulate_auction_with_bundles};
use bundle::Bundle as InternalBundle;

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
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("🧠 Block Engine: Listening for bundles with 200ms auction windows...");

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

        // Parse Redis bundles
        let mut redis_bundles: Vec<Bundle> = bundles_json
            .iter()
            .filter_map(|b| serde_json::from_str(b).ok())
            .collect();

        // Convert Redis bundles to our internal Bundle format for auction processing
        let internal_bundles: Vec<InternalBundle> = redis_bundles
            .iter()
            .map(|b| InternalBundle::new(
                vec![], // Empty transactions for now - would be parsed from b.transactions
                b.tip,
                b.searcher_pubkey.clone(),
            ))
            .collect();

        info!(
            "📦 Processing auction window {} with {} bundles from Redis",
            window_id, internal_bundles.len()
        );

        // Run our sophisticated auction logic with 200ms window simulation
        const MAX_BUNDLES_FOR_BLOCK: usize = 5;
        match simulate_auction_with_bundles(window_id, internal_bundles, MAX_BUNDLES_FOR_BLOCK) {
            Ok(winning_bundles) => {
                // Convert winners back to Redis format for compatibility
                let ordered_bundles: Vec<Bundle> = winning_bundles
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, winner)| {
                        if idx < redis_bundles.len() {
                            // Find matching bundle by tip amount and searcher
                            redis_bundles.iter().find(|rb| 
                                rb.tip == winner.tip_lamports && 
                                rb.searcher_pubkey == winner.searcher_pubkey
                            ).cloned()
                        } else {
                            None
                        }
                    })
                    .collect();

                // Create deterministic ordered hash
                let mut hasher = Sha256::new();
                for b in &ordered_bundles {
                    hasher.update(b.id.as_bytes());
                    hasher.update(b.tip.to_le_bytes());
                }
                let ordered_hash = format!("{:x}", hasher.finalize());

                let block = OrderedBlock {
                    window_id,
                    ordered_bundles: ordered_bundles.clone(),
                    ordered_hash,
                };

                info!(
                    "✅ Built block for window {} with {} winning bundles → hash: {}",
                    window_id,
                    block.ordered_bundles.len(),
                    &block.ordered_hash[..16]
                );

                // Log top bundles with more detail
                for (i, bundle) in ordered_bundles.iter().take(3).enumerate() {
                    info!(
                        "🏆 Winner #{}: Bundle {} from {} with {} lamports tip",
                        i + 1,
                        bundle.id,
                        bundle.searcher_pubkey,
                        bundle.tip
                    );
                }

                // Optional: send to mock validator
                let _ = client.post("http://localhost:4000/submit_block")
                    .json(&block)
                    .send()
                    .await;
            }
            Err(e) => {
                warn!("Auction processing failed for window {}: {}", window_id, e);
                
                // Fallback to simple sorting as before
                redis_bundles.sort_by_key(|b| (b.tip, hash_str(&b.id)));
                let ordered_hash = create_simple_hash(&redis_bundles);
                
                let block = OrderedBlock {
                    window_id,
                    ordered_bundles: redis_bundles,
                    ordered_hash,
                };

                info!(
                    "⚠️ Fallback: Built block for window {} with {} bundles (simple sort)",
                    window_id,
                    block.ordered_bundles.len()
                );
            }
        }

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

// --- helper: create simple hash for fallback ---
fn create_simple_hash(bundles: &[Bundle]) -> String {
    let mut hasher = Sha256::new();
    for b in bundles {
        hasher.update(b.id.as_bytes());
        hasher.update(b.tip.to_le_bytes());
    }
    format!("{:x}", hasher.finalize())
}
