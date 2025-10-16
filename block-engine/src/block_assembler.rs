use crate::bundle::Bundle;
use solana_sdk::{
    hash::Hash,
    pubkey::Pubkey,
    signature::{Signature, Keypair, Signer},
    transaction::Transaction,
};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub slot: u64,
    pub parent_hash: Hash,
    pub blockhash: Hash,
    pub transactions: Vec<Transaction>,
    pub bundles: Vec<Bundle>,
    pub timestamp: u64,
    pub leader_pubkey: Pubkey,
    pub total_fees: u64,
    pub total_tips: u64,
}

#[derive(Debug, Clone)]
pub struct BlockTemplate {
    pub slot: u64,
    pub parent_hash: Hash,
    pub leader_pubkey: Pubkey,
    pub max_transactions: usize,
    pub max_compute_units: u64,
}

pub struct BlockAssembler {
    pub current_slot: u64,
    pub leader_pubkey: Pubkey,
    pub max_transactions_per_block: usize,
    pub max_compute_units_per_block: u64,
}

impl BlockAssembler {
    pub fn new(
        leader_pubkey: Pubkey,
        max_transactions_per_block: usize,
        max_compute_units_per_block: u64,
    ) -> Self {
        Self {
            current_slot: 0,
            leader_pubkey,
            max_transactions_per_block,
            max_compute_units_per_block,
        }
    }

    pub fn create_block_template(&self, slot: u64, parent_hash: Hash) -> BlockTemplate {
        BlockTemplate {
            slot,
            parent_hash,
            leader_pubkey: self.leader_pubkey,
            max_transactions: self.max_transactions_per_block,
            max_compute_units: self.max_compute_units_per_block,
        }
    }

    pub async fn assemble_block(
        &self,
        template: BlockTemplate,
        winning_bundles: Vec<Bundle>,
    ) -> Result<Block> {
        let mut all_transactions = Vec::new();
        let mut total_tips = 0;
        let mut total_compute_units = 0;

        // Process bundles in order of selection (highest tip first)
        let mut included_bundles = Vec::new();
        
        for bundle in winning_bundles {
            let bundle_compute_units = self.estimate_bundle_compute_units(&bundle);
            
            // Check if adding this bundle would exceed limits
            if all_transactions.len() + bundle.transactions.len() > template.max_transactions {
                tracing::warn!("Bundle {} would exceed transaction limit", bundle.id);
                continue;
            }
            
            if total_compute_units + bundle_compute_units > template.max_compute_units {
                tracing::warn!("Bundle {} would exceed compute unit limit", bundle.id);
                continue;
            }

            // Add bundle transactions
            for transaction in &bundle.transactions {
                all_transactions.push(transaction.clone());
            }

            total_tips += bundle.tip_lamports;
            total_compute_units += bundle_compute_units;
            included_bundles.push(bundle);
        }

        // Calculate total fees (simplified - in reality this would be more complex)
        let total_fees = all_transactions.len() as u64 * 5000; // 5000 lamports per transaction

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Ok(Block {
            slot: template.slot,
            parent_hash: template.parent_hash,
            blockhash: Hash::new_unique(), // In reality, this would be computed
            transactions: all_transactions,
            bundles: included_bundles,
            timestamp,
            leader_pubkey: template.leader_pubkey,
            total_fees,
            total_tips,
        })
    }

    fn estimate_bundle_compute_units(&self, bundle: &Bundle) -> u64 {
        // Simplified estimation - in reality would be more sophisticated
        bundle.transactions.len() as u64 * 5000
    }

    pub fn validate_block(&self, block: &Block) -> Result<(), BlockValidationError> {
        // Check transaction count
        if block.transactions.len() > self.max_transactions_per_block {
            return Err(BlockValidationError::TooManyTransactions);
        }

        // Check compute units (simplified)
        let total_compute_units = block.transactions.len() as u64 * 5000;
        if total_compute_units > self.max_compute_units_per_block {
            return Err(BlockValidationError::TooManyComputeUnits);
        }

        // Validate that all bundle transactions are included
        for bundle in &block.bundles {
            for bundle_tx in &bundle.transactions {
                if !block.transactions.contains(bundle_tx) {
                    return Err(BlockValidationError::MissingBundleTransaction);
                }
            }
        }

        Ok(())
    }

    pub fn get_block_stats(&self, block: &Block) -> BlockStats {
        let bundle_count = block.bundles.len();
        let transaction_count = block.transactions.len();
        let avg_tip_per_bundle = if bundle_count > 0 {
            block.total_tips / bundle_count as u64
        } else {
            0
        };

        BlockStats {
            slot: block.slot,
            bundle_count,
            transaction_count,
            total_fees: block.total_fees,
            total_tips: block.total_tips,
            avg_tip_per_bundle,
            timestamp: block.timestamp,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BlockStats {
    pub slot: u64,
    pub bundle_count: usize,
    pub transaction_count: usize,
    pub total_fees: u64,
    pub total_tips: u64,
    pub avg_tip_per_bundle: u64,
    pub timestamp: u64,
}

#[derive(thiserror::Error, Debug)]
pub enum BlockValidationError {
    #[error("Block contains too many transactions")]
    TooManyTransactions,
    #[error("Block exceeds compute unit limit")]
    TooManyComputeUnits,
    #[error("Missing transaction from bundle")]
    MissingBundleTransaction,
    #[error("Invalid block structure: {0}")]
    InvalidStructure(String),
}

// Mock validator client for testing
#[derive(Debug)]
pub struct MockValidatorClient {
    pub submitted_blocks: std::sync::Arc<std::sync::RwLock<Vec<Block>>>,
    pub should_fail: bool,
}

impl MockValidatorClient {
    pub fn new() -> Self {
        Self {
            submitted_blocks: std::sync::Arc::new(std::sync::RwLock::new(Vec::new())),
            should_fail: false,
        }
    }

    pub fn set_failure_mode(&mut self, should_fail: bool) {
        self.should_fail = should_fail;
    }

    pub async fn submit_block(&self, block: Block) -> Result<Signature> {
        if self.should_fail {
            return Err(anyhow::anyhow!("Mock validator client set to fail"));
        }

        let mut blocks = self.submitted_blocks.write().unwrap();
        blocks.push(block);
        
        Ok(Signature::new_unique())
    }

    pub fn get_submitted_blocks(&self) -> Vec<Block> {
        let blocks = self.submitted_blocks.read().unwrap();
        blocks.clone()
    }

    pub fn clear_submitted_blocks(&self) {
        let mut blocks = self.submitted_blocks.write().unwrap();
        blocks.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::Bundle;
    use solana_sdk::{
        signature::Keypair,
        system_instruction,
        transaction::Transaction,
    };

    fn create_test_bundle(tip: u64, tx_count: usize) -> Bundle {
        let keypair = Keypair::new();
        let mut transactions = Vec::new();
        
        for _ in 0..tx_count {
            let instruction = system_instruction::transfer(&keypair.pubkey(), &keypair.pubkey(), 100);
            let transaction = Transaction::new_with_payer(&[instruction], Some(&keypair.pubkey()));
            transactions.push(transaction);
        }
        
        Bundle::new(transactions, tip, keypair.pubkey().to_string())
    }

    #[tokio::test]
    async fn test_assemble_block() {
        let leader = Keypair::new();
        let assembler = BlockAssembler::new(leader.pubkey(), 100, 500_000);
        
        let template = assembler.create_block_template(1, Hash::new_unique());
        let bundles = vec![
            create_test_bundle(2000, 2),
            create_test_bundle(1000, 1),
        ];

        let block = assembler.assemble_block(template, bundles).await.unwrap();
        
        assert_eq!(block.slot, 1);
        assert_eq!(block.bundles.len(), 2);
        assert_eq!(block.transactions.len(), 3);
        assert_eq!(block.total_tips, 3000);
    }

    #[tokio::test]
    async fn test_transaction_limit() {
        let leader = Keypair::new();
        let assembler = BlockAssembler::new(leader.pubkey(), 2, 500_000); // Limit to 2 transactions
        
        let template = assembler.create_block_template(1, Hash::new_unique());
        let bundles = vec![
            create_test_bundle(2000, 2), // This should be included
            create_test_bundle(1000, 2), // This should be rejected due to tx limit
        ];

        let block = assembler.assemble_block(template, bundles).await.unwrap();
        
        assert_eq!(block.bundles.len(), 1);
        assert_eq!(block.transactions.len(), 2);
        assert_eq!(block.total_tips, 2000);
    }

    #[test]
    fn test_validate_block() {
        let leader = Keypair::new();
        let assembler = BlockAssembler::new(leader.pubkey(), 10, 50_000);
        
        let bundle = create_test_bundle(1000, 1);
        let block = Block {
            slot: 1,
            parent_hash: Hash::new_unique(),
            blockhash: Hash::new_unique(),
            transactions: bundle.transactions.clone(),
            bundles: vec![bundle],
            timestamp: 1000,
            leader_pubkey: leader.pubkey(),
            total_fees: 5000,
            total_tips: 1000,
        };

        assert!(assembler.validate_block(&block).is_ok());
    }

    #[tokio::test]
    async fn test_mock_validator_client() {
        let client = MockValidatorClient::new();
        let bundle = create_test_bundle(1000, 1);
        
        let block = Block {
            slot: 1,
            parent_hash: Hash::new_unique(),
            blockhash: Hash::new_unique(),
            transactions: bundle.transactions.clone(),
            bundles: vec![bundle],
            timestamp: 1000,
            leader_pubkey: Keypair::new().pubkey(),
            total_fees: 5000,
            total_tips: 1000,
        };

        let signature = client.submit_block(block.clone()).await.unwrap();
        assert!(!signature.to_string().is_empty());

        let submitted = client.get_submitted_blocks();
        assert_eq!(submitted.len(), 1);
        assert_eq!(submitted[0].slot, block.slot);
    }

    #[tokio::test]
    async fn test_mock_validator_client_failure() {
        let mut client = MockValidatorClient::new();
        client.set_failure_mode(true);
        
        let bundle = create_test_bundle(1000, 1);
        let block = Block {
            slot: 1,
            parent_hash: Hash::new_unique(),
            blockhash: Hash::new_unique(),
            transactions: bundle.transactions.clone(),
            bundles: vec![bundle],
            timestamp: 1000,
            leader_pubkey: Keypair::new().pubkey(),
            total_fees: 5000,
            total_tips: 1000,
        };

        assert!(client.submit_block(block).await.is_err());
    }
}
