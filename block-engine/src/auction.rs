use crate::bundle::Bundle;
use crate::simulator::TransactionSimulator;
use std::collections::BinaryHeap;
use std::cmp::Ordering;
use anyhow::Result;
use tokio::time::{sleep, Duration, Instant};
use tracing::{info, warn, debug};
use uuid::Uuid;

pub struct BundleAuction {
    pub bundles: BinaryHeap<AuctionBundle>,
    pub slot: u64,
    pub simulator: Option<TransactionSimulator>,
}

#[derive(Debug)]
struct AuctionBundle {
    bundle: Bundle,
    priority_score: u64,
}

impl PartialEq for AuctionBundle {
    fn eq(&self, other: &Self) -> bool {
        self.priority_score == other.priority_score
    }
}

impl Eq for AuctionBundle {}

impl PartialOrd for AuctionBundle {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AuctionBundle {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority_score.cmp(&other.priority_score)
    }
}
impl BundleAuction {
    pub fn new(slot: u64) -> Self {
        Self {
            bundles: BinaryHeap::new(),
            slot,
            simulator: None,
        }
    }

    pub fn new_with_simulator(slot: u64, simulator: TransactionSimulator) -> Self {
        Self {
            bundles: BinaryHeap::new(),
            slot,
            simulator: Some(simulator),
        }
    }
    
    pub async fn add_bundle(&mut self, bundle: Bundle) -> Result<()> {
        // If we have a simulator, validate the bundle first
        if let Some(ref simulator) = self.simulator {
            match simulator.validate_bundle(&bundle).await {
                Ok(_) => {
                    let priority_score = bundle.tip_lamports;
                    self.bundles.push(AuctionBundle { bundle, priority_score });
                }
                Err(e) => {
                    tracing::warn!("Bundle {} failed simulation: {}", bundle.id, e);
                    return Err(anyhow::anyhow!("Bundle validation failed: {}", e));
                }
            }
        } else {
            // No simulator, add directly
            let priority_score = bundle.tip_lamports;
            self.bundles.push(AuctionBundle { bundle, priority_score });
        }
        Ok(())
    }
    
    pub fn select_winning_bundles(&mut self, max_bundles: usize) -> Vec<Bundle> {
        let mut winners = Vec::new();
        
        for _ in 0..max_bundles {
            if let Some(winner) = self.bundles.pop() {
                winners.push(winner.bundle);
            } else {
                break;
            }
        }
        
        winners
    }

    pub fn get_auction_stats(&self) -> AuctionStats {
        let total_bundles = self.bundles.len();
        let total_tip_value = self.bundles.iter().map(|b| b.priority_score).sum();
        let highest_tip = self.bundles.peek().map(|b| b.priority_score).unwrap_or(0);
        let avg_tip = if total_bundles > 0 {
            total_tip_value / total_bundles as u64
        } else {
            0
        };

        AuctionStats {
            slot: self.slot,
            total_bundles,
            total_tip_value,
            highest_tip,
            avg_tip,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuctionStats {
    pub slot: u64,
    pub total_bundles: usize,
    pub total_tip_value: u64,
    pub highest_tip: u64,
    pub avg_tip: u64,
}

/// Represents an auction window that collects bundles for 200ms
pub struct AuctionWindow {
    pub window_id: u64,
    pub bundles: Vec<Bundle>,
    pub start_time: Instant,
    pub duration_ms: u64,
    pub max_bundles_for_block: usize,
}

impl AuctionWindow {
    pub fn new(window_id: u64, duration_ms: u64, max_bundles_for_block: usize) -> Self {
        Self {
            window_id,
            bundles: Vec::new(),
            start_time: Instant::now(),
            duration_ms,
            max_bundles_for_block,
        }
    }

    /// Add a bundle to the auction window if it's still open
    pub fn try_add_bundle(&mut self, bundle: Bundle) -> Result<bool> {
        if self.is_window_open() {
            debug!(
                "Adding bundle {} to auction window {} with tip {} lamports",
                bundle.id, self.window_id, bundle.tip_lamports
            );
            self.bundles.push(bundle);
            Ok(true)
        } else {
            debug!(
                "Rejecting bundle {} - auction window {} is closed",
                bundle.id, self.window_id
            );
            Ok(false)
        }
    }

    /// Check if the auction window is still accepting bundles
    pub fn is_window_open(&self) -> bool {
        self.start_time.elapsed().as_millis() < self.duration_ms as u128
    }

    /// Wait for the auction window to close
    pub async fn wait_for_window_close(&self) {
        let elapsed = self.start_time.elapsed().as_millis() as u64;
        if elapsed < self.duration_ms {
            let remaining = self.duration_ms - elapsed;
            debug!(
                "Waiting {}ms for auction window {} to close",
                remaining, self.window_id
            );
            sleep(Duration::from_millis(remaining)).await;
        }
    }

    /// Rank bundles by priority fee (tip_lamports) in descending order
    /// In case of ties, use bundle ID for deterministic ordering
    pub fn rank_bundles_by_priority(&mut self) -> Vec<Bundle> {
        info!(
            "Ranking {} bundles in auction window {} by priority fee",
            self.bundles.len(),
            self.window_id
        );

        // Sort bundles by tip_lamports (descending), then by bundle ID for determinism
        self.bundles.sort_by(|a, b| {
            match b.tip_lamports.cmp(&a.tip_lamports) {
                Ordering::Equal => a.id.cmp(&b.id), // Deterministic tie-breaking
                other => other,
            }
        });

        self.bundles.clone()
    }

    /// Select the top bundles for block inclusion and log the winners
    pub fn select_and_log_winners(&mut self) -> Vec<Bundle> {
        let ranked_bundles = self.rank_bundles_by_priority();
        let winners: Vec<Bundle> = ranked_bundles
            .into_iter()
            .take(self.max_bundles_for_block)
            .collect();

        self.log_auction_results(&winners);
        winners
    }

    /// Log detailed auction results
    fn log_auction_results(&self, winners: &[Bundle]) {
        let total_bundles = self.bundles.len();
        let total_tip_value: u64 = self.bundles.iter().map(|b| b.tip_lamports).sum();
        let avg_tip = if total_bundles > 0 {
            total_tip_value / total_bundles as u64
        } else {
            0
        };

        info!(
            "🏆 Auction Window {} Results: {} total bundles, {} winners selected",
            self.window_id, total_bundles, winners.len()
        );

        info!(
            "📊 Auction Window {} Stats: Total tip value: {} lamports, Average tip: {} lamports",
            self.window_id, total_tip_value, avg_tip
        );

        // Log each winner
        for (rank, winner) in winners.iter().enumerate() {
            info!(
                "🥇 Winner #{}: Bundle {} from searcher {} with tip {} lamports",
                rank + 1,
                winner.id,
                winner.searcher_pubkey,
                winner.tip_lamports
            );
        }

        // Log some stats about non-winning bundles
        if self.bundles.len() > winners.len() {
            let non_winners = &self.bundles[winners.len()..];
            let min_winning_tip = winners.last().map(|w| w.tip_lamports).unwrap_or(0);
            let highest_losing_tip = non_winners.first().map(|b| b.tip_lamports).unwrap_or(0);
            
            info!(
                "📉 {} bundles did not win. Highest losing tip: {} lamports, Minimum winning tip: {} lamports",
                non_winners.len(),
                highest_losing_tip,
                min_winning_tip
            );
        }
    }

    /// Get comprehensive auction statistics
    pub fn get_auction_stats(&self) -> AuctionWindowStats {
        let total_bundles = self.bundles.len();
        let total_tip_value: u64 = self.bundles.iter().map(|b| b.tip_lamports).sum();
        let highest_tip = self.bundles.iter().map(|b| b.tip_lamports).max().unwrap_or(0);
        let lowest_tip = self.bundles.iter().map(|b| b.tip_lamports).min().unwrap_or(0);
        let avg_tip = if total_bundles > 0 {
            total_tip_value / total_bundles as u64
        } else {
            0
        };

        AuctionWindowStats {
            window_id: self.window_id,
            total_bundles,
            total_tip_value,
            highest_tip,
            lowest_tip,
            avg_tip,
            duration_ms: self.duration_ms,
            elapsed_ms: self.start_time.elapsed().as_millis() as u64,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuctionWindowStats {
    pub window_id: u64,
    pub total_bundles: usize,
    pub total_tip_value: u64,
    pub highest_tip: u64,
    pub lowest_tip: u64,
    pub avg_tip: u64,
    pub duration_ms: u64,
    pub elapsed_ms: u64,
}

/// Main auction simulation function that runs a 200ms auction window
/// Collects bundles, ranks them by priority fee, and returns winners
pub async fn simulate_auction_window(
    window_id: u64,
    bundle_receiver: tokio::sync::mpsc::Receiver<Bundle>,
    max_bundles_for_block: usize,
) -> Result<Vec<Bundle>> {
    const AUCTION_DURATION_MS: u64 = 200;
    
    let mut auction_window = AuctionWindow::new(window_id, AUCTION_DURATION_MS, max_bundles_for_block);
    let mut bundle_receiver = bundle_receiver;
    
    info!(
        "🚀 Starting auction window {} for {}ms, accepting up to {} bundles for block",
        window_id, AUCTION_DURATION_MS, max_bundles_for_block
    );

    // Collect bundles for the duration of the auction window
    while auction_window.is_window_open() {
        tokio::select! {
            // Try to receive a bundle
            bundle_result = bundle_receiver.recv() => {
                match bundle_result {
                    Some(bundle) => {
                        if let Err(e) = auction_window.try_add_bundle(bundle) {
                            warn!("Failed to add bundle to auction window: {}", e);
                        }
                    }
                    None => {
                        debug!("Bundle receiver closed, ending auction early");
                        break;
                    }
                }
            }
            // Wait for window to close
            _ = auction_window.wait_for_window_close() => {
                debug!("Auction window {} closed after {}ms", window_id, AUCTION_DURATION_MS);
                break;
            }
        }
    }

    // Select winners and log results
    let winners = auction_window.select_and_log_winners();
    
    info!(
        "✅ Auction window {} completed: {} bundles collected, {} winners selected",
        window_id,
        auction_window.bundles.len(),
        winners.len()
    );

    Ok(winners)
}

/// Alternative simpler function that simulates auction with a pre-collected set of bundles
pub fn simulate_auction_with_bundles(
    window_id: u64,
    bundles: Vec<Bundle>,
    max_bundles_for_block: usize,
) -> Result<Vec<Bundle>> {
    let mut auction_window = AuctionWindow::new(window_id, 200, max_bundles_for_block);
    
    info!(
        "🎯 Simulating auction window {} with {} pre-collected bundles",
        window_id, bundles.len()
    );

    // Add all bundles to the auction window
    for bundle in bundles {
        if let Err(e) = auction_window.try_add_bundle(bundle) {
            warn!("Failed to add bundle to auction simulation: {}", e);
        }
    }

    // Select winners and log results
    let winners = auction_window.select_and_log_winners();
    
    Ok(winners)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auction_window_ranking() {
        let bundles = vec![
            Bundle::new(vec![], 1000000, "searcher_a".to_string()), // 1 SOL
            Bundle::new(vec![], 2000000, "searcher_b".to_string()), // 2 SOL - should be first
            Bundle::new(vec![], 500000, "searcher_c".to_string()),  // 0.5 SOL
            Bundle::new(vec![], 1500000, "searcher_d".to_string()), // 1.5 SOL - should be second
        ];

        let window_id = 123;
        let max_bundles = 2;

        let winners = simulate_auction_with_bundles(window_id, bundles, max_bundles).unwrap();

        assert_eq!(winners.len(), 2);
        assert_eq!(winners[0].tip_lamports, 2000000); // Highest tip first
        assert_eq!(winners[1].tip_lamports, 1500000); // Second highest tip
        assert_eq!(winners[0].searcher_pubkey, "searcher_b");
        assert_eq!(winners[1].searcher_pubkey, "searcher_d");
    }

    #[test]
    fn test_auction_window_deterministic_tiebreaking() {
        // Create bundles with same tip to test deterministic tiebreaking
        let bundle_a = Bundle::new(vec![], 1000000, "searcher_a".to_string());
        let bundle_b = Bundle::new(vec![], 1000000, "searcher_b".to_string());
        
        let bundles = vec![bundle_b.clone(), bundle_a.clone()]; // Reverse order
        
        let winners = simulate_auction_with_bundles(1, bundles, 2).unwrap();
        
        // Should be sorted deterministically by bundle ID when tips are equal
        assert_eq!(winners.len(), 2);
        assert_eq!(winners[0].tip_lamports, 1000000);
        assert_eq!(winners[1].tip_lamports, 1000000);
        
        // The order should be deterministic based on bundle ID
        assert!(winners[0].id < winners[1].id);
    }

    #[tokio::test]
    async fn test_auction_window_timing() {
        let mut window = AuctionWindow::new(456, 50, 5); // 50ms window
        
        // Window should be open initially
        assert!(window.is_window_open());
        
        // Add a bundle
        let bundle = Bundle::new(vec![], 1000000, "test_searcher".to_string());
        assert!(window.try_add_bundle(bundle).unwrap());
        
        // Wait for window to close
        tokio::time::sleep(Duration::from_millis(60)).await;
        
        // Window should be closed now
        assert!(!window.is_window_open());
        
        // Cannot add bundles to closed window
        let late_bundle = Bundle::new(vec![], 2000000, "late_searcher".to_string());
        assert!(!window.try_add_bundle(late_bundle).unwrap());
    }

    #[test]
    fn test_auction_stats() {
        let bundles = vec![
            Bundle::new(vec![], 1000000, "searcher_1".to_string()),
            Bundle::new(vec![], 2000000, "searcher_2".to_string()),
            Bundle::new(vec![], 500000, "searcher_3".to_string()),
        ];

        let mut window = AuctionWindow::new(789, 200, 5);
        for bundle in bundles {
            window.try_add_bundle(bundle).unwrap();
        }

        let stats = window.get_auction_stats();
        assert_eq!(stats.total_bundles, 3);
        assert_eq!(stats.total_tip_value, 3500000);
        assert_eq!(stats.highest_tip, 2000000);
        assert_eq!(stats.lowest_tip, 500000);
        assert_eq!(stats.avg_tip, 1166666); // 3500000 / 3
    }
}