use crate::bundle::Bundle;
use crate::simulator::TransactionSimulator;
use std::collections::BinaryHeap;
use std::cmp::Ordering;
use anyhow::Result;

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