use crate::bundle::Bundle;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};
use uuid::Uuid;
use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub enum PoolEvent {
    BundleAdded(Uuid),
    BundleRemoved(Uuid),
    BundleUpdated(Uuid),
}

pub struct TransactionPool {
    bundles: Arc<RwLock<HashMap<Uuid, Bundle>>>,
    pending_queue: Arc<RwLock<VecDeque<Uuid>>>,
    event_sender: broadcast::Sender<PoolEvent>,
    max_pool_size: usize,
}

impl TransactionPool {
    pub fn new(max_pool_size: usize) -> Self {
        let (event_sender, _) = broadcast::channel(1000);
        
        Self {
            bundles: Arc::new(RwLock::new(HashMap::new())),
            pending_queue: Arc::new(RwLock::new(VecDeque::new())),
            event_sender,
            max_pool_size,
        }
    }

    pub fn add_bundle(&self, bundle: Bundle) -> Result<(), PoolError> {
        let mut bundles = self.bundles.write().unwrap();
        let mut queue = self.pending_queue.write().unwrap();

        // Check pool size limit
        if bundles.len() >= self.max_pool_size {
            return Err(PoolError::PoolFull);
        }

        // Validate bundle before adding
        bundle.validate().map_err(|e| PoolError::InvalidBundle(e.to_string()))?;

        let bundle_id = bundle.id;
        bundles.insert(bundle_id, bundle);
        queue.push_back(bundle_id);

        // Notify listeners
        let _ = self.event_sender.send(PoolEvent::BundleAdded(bundle_id));

        Ok(())
    }

    pub fn get_bundle(&self, id: &Uuid) -> Option<Bundle> {
        let bundles = self.bundles.read().unwrap();
        bundles.get(id).cloned()
    }

    pub fn remove_bundle(&self, id: &Uuid) -> Option<Bundle> {
        let mut bundles = self.bundles.write().unwrap();
        let mut queue = self.pending_queue.write().unwrap();

        if let Some(bundle) = bundles.remove(id) {
            // Remove from queue if present
            queue.retain(|&x| x != *id);
            
            // Notify listeners
            let _ = self.event_sender.send(PoolEvent::BundleRemoved(*id));
            
            Some(bundle)
        } else {
            None
        }
    }

    pub fn get_pending_bundles(&self, count: usize) -> Vec<Bundle> {
        let bundles = self.bundles.read().unwrap();
        let queue = self.pending_queue.read().unwrap();

        queue
            .iter()
            .take(count)
            .filter_map(|id| bundles.get(id).cloned())
            .collect()
    }

    pub fn get_bundles_by_tip_range(&self, min_tip: u64, max_tip: u64) -> Vec<Bundle> {
        let bundles = self.bundles.read().unwrap();
        
        bundles
            .values()
            .filter(|bundle| bundle.tip_lamports >= min_tip && bundle.tip_lamports <= max_tip)
            .cloned()
            .collect()
    }

    pub fn get_stats(&self) -> PoolStats {
        let bundles = self.bundles.read().unwrap();
        let queue = self.pending_queue.read().unwrap();

        let total_bundles = bundles.len();
        let pending_count = queue.len();
        let total_tip_value = bundles.values().map(|b| b.tip_lamports).sum();
        let avg_tip = if total_bundles > 0 {
            total_tip_value / total_bundles as u64
        } else {
            0
        };

        PoolStats {
            total_bundles,
            pending_count,
            total_tip_value,
            avg_tip,
        }
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<PoolEvent> {
        self.event_sender.subscribe()
    }

    pub fn clear(&self) {
        let mut bundles = self.bundles.write().unwrap();
        let mut queue = self.pending_queue.write().unwrap();
        
        bundles.clear();
        queue.clear();
    }
}

#[derive(Debug, Clone)]
pub struct PoolStats {
    pub total_bundles: usize,
    pub pending_count: usize,
    pub total_tip_value: u64,
    pub avg_tip: u64,
}

#[derive(thiserror::Error, Debug)]
pub enum PoolError {
    #[error("Transaction pool is full")]
    PoolFull,
    #[error("Invalid bundle: {0}")]
    InvalidBundle(String),
    #[error("Bundle not found")]
    BundleNotFound,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::Bundle;
    use solana_sdk::{
        signature::{Keypair, Signer},
        system_instruction,
        transaction::Transaction,
    };

    fn create_test_bundle(tip: u64) -> Bundle {
        let keypair = Keypair::new();
        let instruction = system_instruction::transfer(&keypair.pubkey(), &keypair.pubkey(), 100);
        let transaction = Transaction::new_with_payer(&[instruction], Some(&keypair.pubkey()));
        
        Bundle::new(vec![transaction], tip, keypair.pubkey().to_string())
    }

    #[test]
    fn test_add_bundle() {
        let pool = TransactionPool::new(10);
        let bundle = create_test_bundle(1000);
        let bundle_id = bundle.id;

        assert!(pool.add_bundle(bundle).is_ok());
        assert!(pool.get_bundle(&bundle_id).is_some());
    }

    #[test]
    fn test_pool_full() {
        let pool = TransactionPool::new(1);
        let bundle1 = create_test_bundle(1000);
        let bundle2 = create_test_bundle(2000);

        assert!(pool.add_bundle(bundle1).is_ok());
        assert!(matches!(pool.add_bundle(bundle2), Err(PoolError::PoolFull)));
    }

    #[test]
    fn test_remove_bundle() {
        let pool = TransactionPool::new(10);
        let bundle = create_test_bundle(1000);
        let bundle_id = bundle.id;

        pool.add_bundle(bundle).unwrap();
        assert!(pool.remove_bundle(&bundle_id).is_some());
        assert!(pool.get_bundle(&bundle_id).is_none());
    }

    #[test]
    fn test_get_pending_bundles() {
        let pool = TransactionPool::new(10);
        
        for i in 0..5 {
            let bundle = create_test_bundle(1000 + i);
            pool.add_bundle(bundle).unwrap();
        }

        let pending = pool.get_pending_bundles(3);
        assert_eq!(pending.len(), 3);
    }

    #[test]
    fn test_get_bundles_by_tip_range() {
        let pool = TransactionPool::new(10);
        
        pool.add_bundle(create_test_bundle(500)).unwrap();
        pool.add_bundle(create_test_bundle(1000)).unwrap();
        pool.add_bundle(create_test_bundle(1500)).unwrap();
        pool.add_bundle(create_test_bundle(2000)).unwrap();

        let bundles = pool.get_bundles_by_tip_range(1000, 1500);
        assert_eq!(bundles.len(), 2);
        assert!(bundles.iter().all(|b| b.tip_lamports >= 1000 && b.tip_lamports <= 1500));
    }

    #[test]
    fn test_pool_stats() {
        let pool = TransactionPool::new(10);
        
        pool.add_bundle(create_test_bundle(1000)).unwrap();
        pool.add_bundle(create_test_bundle(2000)).unwrap();

        let stats = pool.get_stats();
        assert_eq!(stats.total_bundles, 2);
        assert_eq!(stats.pending_count, 2);
        assert_eq!(stats.total_tip_value, 3000);
        assert_eq!(stats.avg_tip, 1500);
    }
}
