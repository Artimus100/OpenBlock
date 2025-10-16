use crate::bundle::{Bundle, BundleError};
use async_trait::async_trait;
use solana_sdk::{
    account::Account,
    hash::Hash,
    pubkey::Pubkey,
    transaction::Transaction,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    pub success: bool,
    pub logs: Vec<String>,
    pub accounts_accessed: Vec<Pubkey>,
    pub compute_units_consumed: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MockAccount {
    pub pubkey: Pubkey,
    pub account: Account,
}

#[async_trait]
pub trait SolanaRpcClient: Send + Sync {
    async fn simulate_transaction(&self, transaction: &Transaction) -> Result<SimulationResult>;
    async fn get_account(&self, pubkey: &Pubkey) -> Result<Option<Account>>;
    async fn get_latest_blockhash(&self) -> Result<Hash>;
}

pub struct MockSolanaRpcClient {
    pub accounts: HashMap<Pubkey, Account>,
    pub simulation_failures: Vec<String>, // Transaction signatures that should fail
}

impl MockSolanaRpcClient {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            simulation_failures: Vec::new(),
        }
    }

    pub fn add_account(&mut self, pubkey: Pubkey, account: Account) {
        self.accounts.insert(pubkey, account);
    }

    pub fn set_simulation_failure(&mut self, tx_signature: String) {
        self.simulation_failures.push(tx_signature);
    }
}

#[async_trait]
impl SolanaRpcClient for MockSolanaRpcClient {
    async fn simulate_transaction(&self, transaction: &Transaction) -> Result<SimulationResult> {
        let tx_signature = transaction.signatures[0].to_string();
        
        if self.simulation_failures.contains(&tx_signature) {
            return Ok(SimulationResult {
                success: false,
                logs: vec!["Program execution failed".to_string()],
                accounts_accessed: vec![],
                compute_units_consumed: 0,
                error: Some("Instruction failed".to_string()),
            });
        }

        Ok(SimulationResult {
            success: true,
            logs: vec!["Program log: Success".to_string()],
            accounts_accessed: transaction.message.account_keys.clone(),
            compute_units_consumed: 5000,
            error: None,
        })
    }

    async fn get_account(&self, pubkey: &Pubkey) -> Result<Option<Account>> {
        Ok(self.accounts.get(pubkey).cloned())
    }

    async fn get_latest_blockhash(&self) -> Result<Hash> {
        Ok(Hash::new_unique())
    }
}

pub struct TransactionSimulator {
    rpc_client: Box<dyn SolanaRpcClient>,
}

impl TransactionSimulator {
    pub fn new(rpc_client: Box<dyn SolanaRpcClient>) -> Self {
        Self { rpc_client }
    }

    pub async fn simulate_bundle(&self, bundle: &Bundle) -> Result<Vec<SimulationResult>> {
        let mut results = Vec::new();
        
        for transaction in &bundle.transactions {
            let result = self.rpc_client.simulate_transaction(transaction).await?;
            results.push(result);
        }
        
        Ok(results)
    }

    pub async fn validate_bundle(&self, bundle: &Bundle) -> Result<bool, BundleError> {
        // First validate basic bundle constraints
        bundle.validate()?;
        
        // Simulate all transactions
        let simulation_results = self.simulate_bundle(bundle).await
            .map_err(|e| BundleError::SimulationFailed(e.to_string()))?;
        
        // Check if all transactions would succeed
        for result in simulation_results {
            if !result.success {
                return Err(BundleError::SimulationFailed(
                    result.error.unwrap_or_else(|| "Unknown simulation error".to_string())
                ));
            }
        }
        
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::{
        signature::{Keypair, Signer},
        system_instruction,
    };

    #[tokio::test]
    async fn test_mock_rpc_client_success() {
        let mock_client = MockSolanaRpcClient::new();
        let keypair = Keypair::new();
        let transaction = Transaction::new_with_payer(
            &[system_instruction::transfer(&keypair.pubkey(), &Pubkey::new_unique(), 100)],
            Some(&keypair.pubkey()),
        );

        let result = mock_client.simulate_transaction(&transaction).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_mock_rpc_client_failure() {
        let mut mock_client = MockSolanaRpcClient::new();
        let keypair = Keypair::new();
        let mut transaction = Transaction::new_with_payer(
            &[system_instruction::transfer(&keypair.pubkey(), &Pubkey::new_unique(), 100)],
            Some(&keypair.pubkey()),
        );
        
        // Sign the transaction to get a signature
        transaction.sign(&[&keypair], Hash::new_unique());
        let tx_signature = transaction.signatures[0].to_string();
        
        mock_client.set_simulation_failure(tx_signature);
        
        let result = mock_client.simulate_transaction(&transaction).await.unwrap();
        assert!(!result.success);
        assert!(result.error.is_some());
    }
}
