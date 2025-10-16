use serde::{Deserialize, Serialize};
use solana_sdk::transaction::Transaction;
use std::time::SystemTime;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bundle {
    pub id: Uuid,
    pub transactions: Vec<Transaction>,
    pub tip_lamports: u64,
    pub created_at: SystemTime,
    pub searcher_pubkey: String,
}

impl Bundle {
    pub fn new(transactions: Vec<Transaction>, tip_lamports: u64, searcher_pubkey: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            transactions,
            tip_lamports,
            created_at: SystemTime::now(),
            searcher_pubkey,
        }
    }
    
    pub fn validate(&self) -> Result<(), BundleError> {
        if self.transactions.is_empty() {
            return Err(BundleError::EmptyBundle);
        }
    
        if self.transactions.len() > 5 {
            return Err(BundleError::TooManyTransactions);
        }
        
        Ok(())
    }
}

#[derive(thiserror::Error, Debug)]
pub enum BundleError {
    #[error("Bundle cannot be empty")]
    EmptyBundle,
    #[error("Bundle contains too many transactions (max 5)")]
    TooManyTransactions,
    #[error("Simulation failed: {0}")]
    SimulationFailed(String),
}

pub struct BundleEngine {
    rpc_url: String,
}

impl BundleEngine {
    pub async fn new(rpc_url: String) -> anyhow::Result<Self> {
        Ok(Self { rpc_url })
    }
    
    pub async fn start_auction_loop(&mut self) -> anyhow::Result<()> {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(400)).await;
            // Auction logic will go here
        }
    }
}