use crate::block_assembler::{Block, BlockValidationError};
use solana_sdk::signature::Signature;
use std::sync::{Arc, RwLock};
use tokio::time::{sleep, Duration};
use tracing::{info, warn, error};
use anyhow::Result;
use uuid::Uuid;

/// Mock validator that simulates block verification and inclusion
#[derive(Debug, Clone)]
pub struct MockValidator {
    pub validator_id: String,
    pub accepted_blocks: Arc<RwLock<Vec<Block>>>,
    pub rejected_blocks: Arc<RwLock<Vec<(Block, String)>>>,
    pub verification_delay_ms: u64,
    pub failure_rate: f64, // 0.0 = never fail, 1.0 = always fail
    pub max_transactions_per_block: usize,
    pub max_compute_units_per_block: u64,
}

impl MockValidator {
    /// Create a new mock validator with default settings
    pub fn new() -> Self {
        Self {
            validator_id: format!("validator_{}", Uuid::new_v4().to_string()[..8].to_string()),
            accepted_blocks: Arc::new(RwLock::new(Vec::new())),
            rejected_blocks: Arc::new(RwLock::new(Vec::new())),
            verification_delay_ms: 100,
            failure_rate: 0.0,
            max_transactions_per_block: 100,
            max_compute_units_per_block: 1_000_000,
        }
    }

    /// Create a validator with custom failure rate
    pub fn with_failure_rate(failure_rate: f64) -> Self {
        let mut validator = Self::new();
        validator.failure_rate = failure_rate.clamp(0.0, 1.0);
        validator
    }

    /// Create a validator with custom verification delay
    pub fn with_verification_delay(delay_ms: u64) -> Self {
        let mut validator = Self::new();
        validator.verification_delay_ms = delay_ms;
        validator
    }

    /// Create a validator with custom limits
    pub fn with_limits(max_transactions: usize, max_compute_units: u64) -> Self {
        let mut validator = Self::new();
        validator.max_transactions_per_block = max_transactions;
        validator.max_compute_units_per_block = max_compute_units;
        validator
    }

    /// Submit a block for verification and inclusion
    pub async fn submit_block(&self, block: Block) -> Result<BlockSubmissionResult> {
        info!(
            "🔍 Validator {} received block for slot {} with {} transactions and {} bundles",
            self.validator_id,
            block.slot,
            block.transactions.len(),
            block.bundles.len()
        );

        // Simulate verification delay
        sleep(Duration::from_millis(self.verification_delay_ms)).await;

        // Perform validation checks
        match self.validate_block(&block).await {
            Ok(_) => {
                // Simulate random failure based on failure rate
                if self.should_fail() {
                    let reason = "Random validation failure".to_string();
                    self.reject_block(block, reason.clone()).await;
                    Ok(BlockSubmissionResult::Rejected { reason })
                } else {
                    let signature = self.accept_block(block).await;
                    Ok(BlockSubmissionResult::Accepted { signature })
                }
            }
            Err(validation_error) => {
                let reason = validation_error.to_string();
                self.reject_block(block, reason.clone()).await;
                Ok(BlockSubmissionResult::Rejected { reason })
            }
        }
    }

    /// Validate the block structure and contents
    async fn validate_block(&self, block: &Block) -> Result<(), BlockValidationError> {
        info!(
            "⚡ Validator {} validating block for slot {} (hash: {})",
            self.validator_id,
            block.slot,
            hex::encode(&block.blockhash.to_bytes()[..8])
        );

        // Check transaction count
        if block.transactions.len() > self.max_transactions_per_block {
            warn!(
                "❌ Block validation failed: too many transactions ({} > {})",
                block.transactions.len(),
                self.max_transactions_per_block
            );
            return Err(BlockValidationError::TooManyTransactions);
        }

        // Check compute units (simplified estimation)
        let estimated_compute_units = block.transactions.len() as u64 * 5000;
        if estimated_compute_units > self.max_compute_units_per_block {
            warn!(
                "❌ Block validation failed: too many compute units ({} > {})",
                estimated_compute_units,
                self.max_compute_units_per_block
            );
            return Err(BlockValidationError::TooManyComputeUnits);
        }

        // Validate that all bundle transactions are included in the block
        for bundle in &block.bundles {
            for bundle_tx in &bundle.transactions {
                if !block.transactions.contains(bundle_tx) {
                    warn!(
                        "❌ Block validation failed: missing transaction from bundle {}",
                        bundle.id
                    );
                    return Err(BlockValidationError::MissingBundleTransaction);
                }
            }
        }

        // Check basic block structure
        if block.timestamp == 0 {
            warn!("❌ Block validation failed: invalid timestamp");
            return Err(BlockValidationError::InvalidStructure("Invalid timestamp".to_string()));
        }

        info!("✅ Block validation passed for slot {}", block.slot);
        Ok(())
    }

    /// Accept a block and record it
    async fn accept_block(&self, block: Block) -> Signature {
        let signature = Signature::new_unique();
        
        info!(
            "🎉 Validator {} ACCEPTED block for slot {} with signature {}",
            self.validator_id,
            block.slot,
            signature
        );

        info!(
            "📊 Block stats: {} transactions, {} bundles, {} total fees, {} total tips",
            block.transactions.len(),
            block.bundles.len(),
            block.total_fees,
            block.total_tips
        );

        // Log bundle details
        for (i, bundle) in block.bundles.iter().enumerate() {
            info!(
                "💎 Bundle #{}: {} from {} with {} transactions and {} tip",
                i + 1,
                bundle.id,
                bundle.searcher_pubkey,
                bundle.transactions.len(),
                bundle.tip_lamports
            );
        }

        // Store accepted block
        let mut accepted = self.accepted_blocks.write().unwrap();
        accepted.push(block);

        signature
    }

    /// Reject a block and record the reason
    async fn reject_block(&self, block: Block, reason: String) {
        error!(
            "❌ Validator {} REJECTED block for slot {}: {}",
            self.validator_id,
            block.slot,
            reason
        );

        error!(
            "🚫 Rejected block had {} transactions, {} bundles, {} total fees",
            block.transactions.len(),
            block.bundles.len(),
            block.total_fees
        );

        // Store rejected block with reason
        let mut rejected = self.rejected_blocks.write().unwrap();
        rejected.push((block, reason));
    }

    /// Determine if this submission should fail based on failure rate
    fn should_fail(&self) -> bool {
        if self.failure_rate <= 0.0 {
            return false;
        }
        if self.failure_rate >= 1.0 {
            return true;
        }
        
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        use std::time::{SystemTime, UNIX_EPOCH};
        
        let mut hasher = DefaultHasher::new();
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos().hash(&mut hasher);
        let random_value = (hasher.finish() % 1000) as f64 / 1000.0;
        
        random_value < self.failure_rate
    }

    /// Get statistics about this validator's performance
    pub fn get_stats(&self) -> ValidatorStats {
        let accepted = self.accepted_blocks.read().unwrap();
        let rejected = self.rejected_blocks.read().unwrap();
        
        let total_blocks = accepted.len() + rejected.len();
        let acceptance_rate = if total_blocks > 0 {
            accepted.len() as f64 / total_blocks as f64
        } else {
            0.0
        };

        let total_fees_processed = accepted.iter().map(|b| b.total_fees).sum();
        let total_tips_processed = accepted.iter().map(|b| b.total_tips).sum();
        let total_transactions_processed = accepted.iter().map(|b| b.transactions.len() as u64).sum();

        ValidatorStats {
            validator_id: self.validator_id.clone(),
            blocks_accepted: accepted.len(),
            blocks_rejected: rejected.len(),
            total_blocks_processed: total_blocks,
            acceptance_rate,
            total_fees_processed,
            total_tips_processed,
            total_transactions_processed,
        }
    }

    /// Clear all stored blocks (useful for testing)
    pub fn clear_history(&self) {
        let mut accepted = self.accepted_blocks.write().unwrap();
        let mut rejected = self.rejected_blocks.write().unwrap();
        accepted.clear();
        rejected.clear();
        
        info!("🧹 Validator {} cleared all block history", self.validator_id);
    }

    /// Get all accepted blocks
    pub fn get_accepted_blocks(&self) -> Vec<Block> {
        let accepted = self.accepted_blocks.read().unwrap();
        accepted.clone()
    }

    /// Get all rejected blocks with reasons
    pub fn get_rejected_blocks(&self) -> Vec<(Block, String)> {
        let rejected = self.rejected_blocks.read().unwrap();
        rejected.clone()
    }
}

impl Default for MockValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of a block submission to the validator
#[derive(Debug, Clone)]
pub enum BlockSubmissionResult {
    Accepted { signature: Signature },
    Rejected { reason: String },
}

/// Statistics about validator performance
#[derive(Debug, Clone)]
pub struct ValidatorStats {
    pub validator_id: String,
    pub blocks_accepted: usize,
    pub blocks_rejected: usize,
    pub total_blocks_processed: usize,
    pub acceptance_rate: f64,
    pub total_fees_processed: u64,
    pub total_tips_processed: u64,
    pub total_transactions_processed: u64,
}

/// A network of multiple validators for more realistic simulation
#[derive(Debug)]
pub struct ValidatorNetwork {
    pub validators: Vec<MockValidator>,
}

impl ValidatorNetwork {
    /// Create a network with multiple validators
    pub fn new(count: usize) -> Self {
        let mut validators = Vec::new();
        
        for i in 0..count {
            let mut validator = MockValidator::new();
            validator.validator_id = format!("validator_{}", i);
            
            // Add some variety to the validators
            match i % 3 {
                0 => validator.failure_rate = 0.05, // 5% failure rate
                1 => validator.failure_rate = 0.10, // 10% failure rate
                _ => validator.failure_rate = 0.02, // 2% failure rate
            }
            
            validators.push(validator);
        }
        
        Self { validators }
    }

    /// Submit a block to all validators and return results
    pub async fn submit_block_to_network(&self, block: Block) -> Vec<(String, BlockSubmissionResult)> {
        info!(
            "🌐 Submitting block for slot {} to network of {} validators",
            block.slot,
            self.validators.len()
        );

        let mut results = Vec::new();
        
        // Submit to all validators concurrently
        let futures: Vec<_> = self.validators.iter().map(|validator| {
            let block_clone = block.clone();
            let validator_id = validator.validator_id.clone();
            async move {
                let result = validator.submit_block(block_clone).await.unwrap();
                (validator_id, result)
            }
        }).collect();

        // Wait for all results
        for future in futures {
            results.push(future.await);
        }

        // Log network consensus
        let accepted_count = results.iter().filter(|(_, result)| {
            matches!(result, BlockSubmissionResult::Accepted { .. })
        }).count();
        
        let consensus_rate = accepted_count as f64 / self.validators.len() as f64;
        
        info!(
            "🗳️ Network consensus: {}/{} validators accepted block ({}% acceptance)",
            accepted_count,
            self.validators.len(),
            (consensus_rate * 100.0) as u32
        );

        results
    }

    /// Get aggregate statistics for the entire network
    pub fn get_network_stats(&self) -> Vec<ValidatorStats> {
        self.validators.iter().map(|v| v.get_stats()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::Bundle;
    use solana_sdk::{hash::Hash, pubkey::Pubkey};

    fn create_test_block() -> Block {
        let bundle = Bundle::new(
            vec![], // Empty transactions for test
            1000000,
            "test_searcher".to_string(),
        );

        Block {
            slot: 12345,
            parent_hash: Hash::new_unique(),
            blockhash: Hash::new_unique(),
            transactions: vec![],
            bundles: vec![bundle],
            timestamp: 1000,
            leader_pubkey: Pubkey::new_unique(),
            total_fees: 1000000,
            total_tips: 1000000,
        }
    }

    #[tokio::test]
    async fn test_validator_accepts_valid_block() {
        let validator = MockValidator::new();
        let block = create_test_block();

        let result = validator.submit_block(block).await.unwrap();
        
        match result {
            BlockSubmissionResult::Accepted { signature } => {
                assert!(!signature.to_string().is_empty());
            }
            BlockSubmissionResult::Rejected { .. } => {
                panic!("Expected block to be accepted");
            }
        }

        let stats = validator.get_stats();
        assert_eq!(stats.blocks_accepted, 1);
        assert_eq!(stats.blocks_rejected, 0);
    }

    #[tokio::test]
    async fn test_validator_rejects_invalid_block() {
        let validator = MockValidator::with_limits(0, 1000); // No transactions allowed
        let mut block = create_test_block();
        
        // Add a transaction to make it invalid
        use solana_sdk::{instruction::Instruction, message::Message, transaction::Transaction, signature::Signature};
        let instruction = Instruction::new_with_bytes(Pubkey::new_unique(), &[1, 2, 3], vec![]);
        let message = Message::new(&[instruction], Some(&Pubkey::new_unique()));
        let transaction = Transaction { signatures: vec![Signature::default()], message };
        block.transactions.push(transaction);

        let result = validator.submit_block(block).await.unwrap();
        
        match result {
            BlockSubmissionResult::Rejected { reason } => {
                assert!(reason.contains("too many transactions"));
            }
            BlockSubmissionResult::Accepted { .. } => {
                panic!("Expected block to be rejected");
            }
        }

        let stats = validator.get_stats();
        assert_eq!(stats.blocks_accepted, 0);
        assert_eq!(stats.blocks_rejected, 1);
    }

    #[tokio::test]
    async fn test_validator_with_failure_rate() {
        let validator = MockValidator::with_failure_rate(1.0); // Always fail
        let block = create_test_block();

        let result = validator.submit_block(block).await.unwrap();
        
        match result {
            BlockSubmissionResult::Rejected { .. } => {
                // Expected
            }
            BlockSubmissionResult::Accepted { .. } => {
                panic!("Expected block to be rejected with 100% failure rate");
            }
        }
    }

    #[tokio::test]
    async fn test_validator_network() {
        let network = ValidatorNetwork::new(3);
        let block = create_test_block();

        let results = network.submit_block_to_network(block).await;
        
        assert_eq!(results.len(), 3);
        
        // At least some should be accepted (given low failure rates)
        let accepted_count = results.iter().filter(|(_, result)| {
            matches!(result, BlockSubmissionResult::Accepted { .. })
        }).count();
        
        assert!(accepted_count > 0, "At least one validator should accept the block");
    }
}
