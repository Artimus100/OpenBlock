use crate::{
    bundle::Bundle,
    transaction_pool::TransactionPool,
    auction::BundleAuction,
    simulator::{MockSolanaRpcClient, TransactionSimulator},
    block_assembler::{BlockAssembler, MockValidatorClient},
};
use proptest::prelude::*;
use solana_sdk::{
    hash::Hash,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
    pubkey::Pubkey,
};
use std::collections::HashSet;

// Property-based test helpers
fn arb_bundle() -> impl Strategy<Value = Bundle> {
    (
        1u64..=10_000_000,  // tip_lamports
        1usize..=5,         // number of transactions
        "[a-zA-Z0-9]{44}",  // searcher_pubkey
    ).prop_map(|(tip, tx_count, searcher_pubkey)| {
        let keypair = Keypair::new();
        let mut transactions = Vec::new();
        
        for _ in 0..tx_count {
            let instruction = system_instruction::transfer(
                &keypair.pubkey(), 
                &Pubkey::new_unique(), 
                100
            );
            let mut transaction = Transaction::new_with_payer(&[instruction], Some(&keypair.pubkey()));
            transaction.sign(&[&keypair], Hash::new_unique());
            transactions.push(transaction);
        }
        
        Bundle::new(transactions, tip, searcher_pubkey)
    })
}

#[cfg(test)]
mod property_tests {
    use super::*;

    proptest! {
        #[test]
        fn test_bundle_validation_properties(bundle in arb_bundle()) {
            // Property: Valid bundles should always pass validation
            if bundle.transactions.len() > 0 && bundle.transactions.len() <= 5 {
                assert!(bundle.validate().is_ok());
            }
            
            // Property: Bundle tip should be non-negative
            assert!(bundle.tip_lamports >= 0);
            
            // Property: Bundle should have a valid UUID
            assert!(!bundle.id.to_string().is_empty());
        }

        #[test]
        fn test_transaction_pool_properties(
            bundles in prop::collection::vec(arb_bundle(), 1..=100),
            pool_size in 10usize..=1000
        ) {
            let pool = TransactionPool::new(pool_size);
            let mut added_count = 0;
            
            for bundle in bundles {
                if added_count < pool_size && bundle.validate().is_ok() {
                    match pool.add_bundle(bundle) {
                        Ok(_) => added_count += 1,
                        Err(_) => {} // Pool might be full
                    }
                }
            }
            
            let stats = pool.get_stats();
            
            // Property: Pool should never exceed its maximum size
            assert!(stats.total_bundles <= pool_size);
            
            // Property: Total tip value should be sum of all bundle tips
            let pending_bundles = pool.get_pending_bundles(stats.total_bundles);
            let calculated_total: u64 = pending_bundles.iter().map(|b| b.tip_lamports).sum();
            assert_eq!(stats.total_tip_value, calculated_total);
            
            // Property: Average tip should be reasonable
            if stats.total_bundles > 0 {
                assert_eq!(stats.avg_tip, stats.total_tip_value / stats.total_bundles as u64);
            } else {
                assert_eq!(stats.avg_tip, 0);
            }
        }

        #[tokio::test]
        async fn test_auction_ordering_properties(
            mut tips in prop::collection::vec(1u64..=1_000_000, 2..=50)
        ) {
            // Ensure we have unique tips for this test
            tips.sort();
            tips.dedup();
            
            if tips.len() < 2 {
                return Ok(());
            }
            
            let mock_rpc = Box::new(MockSolanaRpcClient::new());
            let simulator = TransactionSimulator::new(mock_rpc);
            let mut auction = BundleAuction::new_with_simulator(1, simulator);
            
            let mut bundles = Vec::new();
            for tip in &tips {
                let keypair = Keypair::new();
                let instruction = system_instruction::transfer(&keypair.pubkey(), &keypair.pubkey(), 100);
                let mut transaction = Transaction::new_with_payer(&[instruction], Some(&keypair.pubkey()));
                transaction.sign(&[&keypair], Hash::new_unique());
                
                let bundle = Bundle::new(vec![transaction], *tip, keypair.pubkey().to_string());
                bundles.push(bundle.clone());
                auction.add_bundle(bundle).await.unwrap();
            }
            
            let winners = auction.select_winning_bundles(tips.len());
            
            // Property: Winners should be ordered by tip (highest first)
            for i in 1..winners.len() {
                assert!(winners[i-1].tip_lamports >= winners[i].tip_lamports,
                       "Winners not ordered by tip: {} < {}", 
                       winners[i-1].tip_lamports, winners[i].tip_lamports);
            }
            
            // Property: All winners should come from the original bundle set
            let original_tips: HashSet<u64> = bundles.iter().map(|b| b.tip_lamports).collect();
            for winner in &winners {
                assert!(original_tips.contains(&winner.tip_lamports));
            }
        }

        #[tokio::test]
        async fn test_block_assembler_properties(
            bundles in prop::collection::vec(arb_bundle(), 1..=20),
            max_transactions in 5usize..=100,
            max_compute_units in 50_000u64..=1_000_000
        ) {
            let leader = Keypair::new();
            let assembler = BlockAssembler::new(leader.pubkey(), max_transactions, max_compute_units);
            let template = assembler.create_block_template(1, Hash::new_unique());
            
            let valid_bundles: Vec<Bundle> = bundles.into_iter()
                .filter(|b| b.validate().is_ok())
                .collect();
            
            if valid_bundles.is_empty() {
                return Ok(());
            }
            
            let block = assembler.assemble_block(template, valid_bundles).await.unwrap();
            
            // Property: Block should not exceed transaction limit
            assert!(block.transactions.len() <= max_transactions);
            
            // Property: All bundle transactions should be included in block
            for bundle in &block.bundles {
                for tx in &bundle.transactions {
                    assert!(block.transactions.contains(tx), 
                           "Bundle transaction not found in block");
                }
            }
            
            // Property: Total tips should equal sum of included bundle tips
            let expected_tips: u64 = block.bundles.iter().map(|b| b.tip_lamports).sum();
            assert_eq!(block.total_tips, expected_tips);
            
            // Property: Block should pass validation
            assert!(assembler.validate_block(&block).is_ok());
        }

        #[test]
        fn test_pool_tip_range_properties(
            bundles in prop::collection::vec(arb_bundle(), 1..=50),
            min_tip in 0u64..=5_000_000,
            tip_range in 1u64..=5_000_000
        ) {
            let max_tip = min_tip + tip_range;
            let pool = TransactionPool::new(100);
            
            for bundle in bundles {
                if bundle.validate().is_ok() {
                    let _ = pool.add_bundle(bundle);
                }
            }
            
            let filtered_bundles = pool.get_bundles_by_tip_range(min_tip, max_tip);
            
            // Property: All returned bundles should be within the specified range
            for bundle in &filtered_bundles {
                assert!(bundle.tip_lamports >= min_tip && bundle.tip_lamports <= max_tip,
                       "Bundle tip {} not in range [{}, {}]", 
                       bundle.tip_lamports, min_tip, max_tip);
            }
            
            // Property: No bundles in range should be missing from results
            let all_bundles = pool.get_pending_bundles(1000);
            let expected_count = all_bundles.iter()
                .filter(|b| b.tip_lamports >= min_tip && b.tip_lamports <= max_tip)
                .count();
            assert_eq!(filtered_bundles.len(), expected_count);
        }

        #[tokio::test]
        async fn test_simulation_consistency_properties(
            bundle in arb_bundle()
        ) {
            let mock_rpc = Box::new(MockSolanaRpcClient::new());
            let simulator = TransactionSimulator::new(mock_rpc);
            
            if bundle.validate().is_err() {
                return Ok(());
            }
            
            // Property: Simulation should be deterministic
            let result1 = simulator.simulate_bundle(&bundle).await.unwrap();
            let result2 = simulator.simulate_bundle(&bundle).await.unwrap();
            
            assert_eq!(result1.len(), result2.len());
            for (r1, r2) in result1.iter().zip(result2.iter()) {
                assert_eq!(r1.success, r2.success);
                assert_eq!(r1.compute_units_consumed, r2.compute_units_consumed);
            }
            
            // Property: Number of simulation results should match number of transactions
            assert_eq!(result1.len(), bundle.transactions.len());
        }

        #[tokio::test]
        async fn test_validator_client_properties(
            bundles in prop::collection::vec(arb_bundle(), 1..=10)
        ) {
            let client = MockValidatorClient::new();
            let leader = Keypair::new();
            let assembler = BlockAssembler::new(leader.pubkey(), 100, 1_000_000);
            
            let valid_bundles: Vec<Bundle> = bundles.into_iter()
                .filter(|b| b.validate().is_ok())
                .collect();
            
            if valid_bundles.is_empty() {
                return Ok(());
            }
            
            let template = assembler.create_block_template(1, Hash::new_unique());
            let block = assembler.assemble_block(template, valid_bundles).await.unwrap();
            
            let signature = client.submit_block(block.clone()).await.unwrap();
            
            // Property: Submission should return a valid signature
            assert!(!signature.to_string().is_empty());
            
            // Property: Submitted block should be recorded
            let submitted_blocks = client.get_submitted_blocks();
            assert!(!submitted_blocks.is_empty());
            assert_eq!(submitted_blocks.last().unwrap().slot, block.slot);
        }
    }
}

// Chaos engineering tests
#[cfg(test)]
mod chaos_tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::time::timeout;

    #[tokio::test]
    async fn test_high_memory_pressure() {
        // Test system behavior under high memory pressure
        let pool = TransactionPool::new(10000);
        
        // Create many large bundles
        for i in 0..1000 {
            let bundle = Bundle::new(
                vec![Transaction::default(); 5], // Maximum transactions per bundle
                i * 1000,
                format!("searcher_{}", i)
            );
            
            if pool.add_bundle(bundle).is_err() {
                break; // Pool is full
            }
        }
        
        // System should still be responsive
        let stats = pool.get_stats();
        assert!(stats.total_bundles > 0);
    }

    #[tokio::test]
    async fn test_concurrent_access_chaos() {
        let pool = Arc::new(TransactionPool::new(1000));
        let mut handles = Vec::new();
        
        // Spawn many concurrent tasks
        for i in 0..50 {
            let pool_clone = Arc::clone(&pool);
            let handle = tokio::spawn(async move {
                for j in 0..20 {
                    let bundle = Bundle::new(
                        vec![Transaction::default()],
                        (i * 100 + j) as u64,
                        format!("searcher_{}_{}", i, j)
                    );
                    
                    let _ = pool_clone.add_bundle(bundle);
                    
                    // Random operations
                    if j % 3 == 0 {
                        let _ = pool_clone.get_pending_bundles(10);
                    }
                    if j % 5 == 0 {
                        let _ = pool_clone.get_stats();
                    }
                }
            });
            handles.push(handle);
        }
        
        // All tasks should complete without panicking
        for handle in handles {
            timeout(Duration::from_secs(10), handle)
                .await
                .expect("Task timed out")
                .expect("Task panicked");
        }
        
        // Pool should be in a consistent state
        let stats = pool.get_stats();
        assert!(stats.total_bundles <= 1000);
    }

    #[tokio::test]
    async fn test_simulation_failure_cascade() {
        let mut mock_rpc = MockSolanaRpcClient::new();
        
        // Create a bundle with a transaction that will fail
        let keypair = Keypair::new();
        let instruction = system_instruction::transfer(&keypair.pubkey(), &keypair.pubkey(), 100);
        let mut transaction = Transaction::new_with_payer(&[instruction], Some(&keypair.pubkey()));
        transaction.sign(&[&keypair], Hash::new_unique());
        
        let failing_bundle = Bundle::new(
            vec![transaction.clone()],
            1000000,
            keypair.pubkey().to_string()
        );
        
        // Set up the mock to fail this specific transaction
        mock_rpc.set_simulation_failure(transaction.signatures[0].to_string());
        
        let simulator = TransactionSimulator::new(Box::new(mock_rpc));
        let mut auction = BundleAuction::new_with_simulator(1, simulator);
        
        // Try to add the failing bundle - should be rejected
        let result = auction.add_bundle(failing_bundle).await;
        assert!(result.is_err());
        
        // Auction should still be functional for valid bundles
        let good_bundle = Bundle::new(
            vec![Transaction::default()],
            2000000,
            "good_searcher".to_string()
        );
        
        let result = auction.add_bundle(good_bundle).await;
        assert!(result.is_ok());
    }
}
