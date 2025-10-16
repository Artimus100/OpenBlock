use crate::{
    auction::BundleAuction,
    block_assembler::{BlockAssembler, MockValidatorClient},
    bundle::Bundle,
    simulator::{MockSolanaRpcClient, TransactionSimulator},
    transaction_pool::{TransactionPool, PoolEvent},
};
use solana_sdk::{
    hash::Hash,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
    pubkey::Pubkey,
};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Barrier;

// Helper function to create test bundles
pub fn create_test_bundle_with_keypair(tip: u64, tx_count: usize, keypair: &Keypair) -> Bundle {
    let mut transactions = Vec::new();
    
    for _ in 0..tx_count {
        let instruction = system_instruction::transfer(&keypair.pubkey(), &Pubkey::new_unique(), 100);
        let mut transaction = Transaction::new_with_payer(&[instruction], Some(&keypair.pubkey()));
        transaction.sign(&[keypair], Hash::new_unique());
        transactions.push(transaction);
    }
    
    Bundle::new(transactions, tip, keypair.pubkey().to_string())
}

pub fn create_test_bundle(tip: u64, tx_count: usize) -> Bundle {
    let keypair = Keypair::new();
    create_test_bundle_with_keypair(tip, tx_count, &keypair)
}

// Comprehensive integration test
#[tokio::test]
async fn test_full_pipeline_integration() {
    // Setup components
    let pool = TransactionPool::new(100);
    let mock_rpc = Box::new(MockSolanaRpcClient::new());
    let simulator = TransactionSimulator::new(mock_rpc);
    let mut auction = BundleAuction::new_with_simulator(1, simulator);
    let leader = Keypair::new();
    let assembler = BlockAssembler::new(leader.pubkey(), 50, 500_000);
    let validator_client = MockValidatorClient::new();

    // Create test bundles with varying tips
    let bundles = vec![
        create_test_bundle(5000, 2), // Highest tip
        create_test_bundle(3000, 1), // Medium tip
        create_test_bundle(1000, 1), // Lowest tip
        create_test_bundle(4000, 3), // High tip, more transactions
    ];

    // Add bundles to pool
    for bundle in &bundles {
        pool.add_bundle(bundle.clone()).expect("Failed to add bundle to pool");
    }

    // Get bundles from pool and add to auction
    let pending_bundles = pool.get_pending_bundles(10);
    for bundle in pending_bundles {
        auction.add_bundle(bundle).await.expect("Failed to add bundle to auction");
    }

    // Select winning bundles (should be ordered by tip)
    let winners = auction.select_winning_bundles(3);
    assert_eq!(winners.len(), 3);
    assert_eq!(winners[0].tip_lamports, 5000); // Highest tip first
    assert_eq!(winners[1].tip_lamports, 4000); // Second highest
    assert_eq!(winners[2].tip_lamports, 3000); // Third highest

    // Assemble block
    let template = assembler.create_block_template(1, Hash::new_unique());
    let block = assembler.assemble_block(template, winners).await.expect("Failed to assemble block");

    // Validate block
    assembler.validate_block(&block).expect("Block validation failed");

    // Submit to validator
    validator_client.submit_block(block.clone()).await.expect("Failed to submit block");

    // Verify submission
    let submitted_blocks = validator_client.get_submitted_blocks();
    assert_eq!(submitted_blocks.len(), 1);
    assert_eq!(submitted_blocks[0].slot, block.slot);
    assert_eq!(submitted_blocks[0].total_tips, 12000); // 5000 + 4000 + 3000
}

#[tokio::test]
async fn test_auction_filters_failed_simulations() {
    let mut mock_rpc = MockSolanaRpcClient::new();
    let keypair = Keypair::new();
    
    // Create a bundle that will fail simulation
    let failing_bundle = create_test_bundle_with_keypair(10000, 1, &keypair); // High tip but will fail
    let failing_tx_sig = failing_bundle.transactions[0].signatures[0].to_string();
    mock_rpc.set_simulation_failure(failing_tx_sig);
    
    let simulator = TransactionSimulator::new(Box::new(mock_rpc));
    let mut auction = BundleAuction::new_with_simulator(1, simulator);

    // Add a good bundle
    let good_bundle = create_test_bundle(5000, 1);
    auction.add_bundle(good_bundle).await.expect("Good bundle should be added");

    // Try to add the failing bundle
    let result = auction.add_bundle(failing_bundle).await;
    assert!(result.is_err(), "Failing bundle should be rejected");

    // Only the good bundle should be in the auction
    let winners = auction.select_winning_bundles(10);
    assert_eq!(winners.len(), 1);
    assert_eq!(winners[0].tip_lamports, 5000);
}

#[tokio::test]
async fn test_transaction_pool_concurrency() {
    let pool = Arc::new(TransactionPool::new(1000));
    let num_concurrent_submitters = 10;
    let bundles_per_submitter = 5;

    let barrier = Arc::new(Barrier::new(num_concurrent_submitters));
    let mut handles = Vec::new();

    // Spawn concurrent bundle submitters
    for submitter_id in 0..num_concurrent_submitters {
        let pool_clone = Arc::clone(&pool);
        let barrier_clone = Arc::clone(&barrier);
        
        let handle = tokio::spawn(async move {
            barrier_clone.wait().await;
            
            for i in 0..bundles_per_submitter {
                let tip = (submitter_id * 1000 + i * 100) as u64;
                let bundle = create_test_bundle(tip, 1);
                pool_clone.add_bundle(bundle).expect("Failed to add bundle");
            }
        });
        
        handles.push(handle);
    }

    // Wait for all submitters to complete
    for handle in handles {
        handle.await.expect("Submitter task failed");
    }

    // Verify all bundles were added
    let stats = pool.get_stats();
    assert_eq!(stats.total_bundles, num_concurrent_submitters * bundles_per_submitter);
}

#[tokio::test]
async fn test_pool_event_notifications() {
    let pool = TransactionPool::new(10);
    let mut event_receiver = pool.subscribe_events();

    // Add a bundle and check for event
    let bundle = create_test_bundle(1000, 1);
    let bundle_id = bundle.id;
    
    pool.add_bundle(bundle).expect("Failed to add bundle");
    
    // Should receive BundleAdded event
    let event = tokio::time::timeout(Duration::from_millis(100), event_receiver.recv())
        .await
        .expect("Timeout waiting for event")
        .expect("Failed to receive event");
    
    match event {
        PoolEvent::BundleAdded(id) => assert_eq!(id, bundle_id),
        _ => panic!("Expected BundleAdded event"),
    }

    // Remove bundle and check for event
    pool.remove_bundle(&bundle_id);
    
    let event = tokio::time::timeout(Duration::from_millis(100), event_receiver.recv())
        .await
        .expect("Timeout waiting for event")
        .expect("Failed to receive event");
    
    match event {
        PoolEvent::BundleRemoved(id) => assert_eq!(id, bundle_id),
        _ => panic!("Expected BundleRemoved event"),
    }
}

#[tokio::test]
async fn test_end_to_end_latency_benchmark() {
    let start_time = Instant::now();
    
    // Setup
    let pool = TransactionPool::new(100);
    let mock_rpc = Box::new(MockSolanaRpcClient::new());
    let simulator = TransactionSimulator::new(mock_rpc);
    let mut auction = BundleAuction::new_with_simulator(1, simulator);
    let leader = Keypair::new();
    let assembler = BlockAssembler::new(leader.pubkey(), 50, 500_000);
    let validator_client = MockValidatorClient::new();

    let setup_time = start_time.elapsed();

    // Bundle submission phase
    let submission_start = Instant::now();
    let bundle = create_test_bundle(1000, 2);
    pool.add_bundle(bundle.clone()).expect("Failed to add bundle");
    let submission_time = submission_start.elapsed();

    // Auction phase
    let auction_start = Instant::now();
    auction.add_bundle(bundle).await.expect("Failed to add to auction");
    let winners = auction.select_winning_bundles(1);
    let auction_time = auction_start.elapsed();

    // Block assembly phase
    let assembly_start = Instant::now();
    let template = assembler.create_block_template(1, Hash::new_unique());
    let block = assembler.assemble_block(template, winners).await.expect("Failed to assemble block");
    let assembly_time = assembly_start.elapsed();

    // Validator submission phase
    let validator_start = Instant::now();
    validator_client.submit_block(block).await.expect("Failed to submit block");
    let validator_time = validator_start.elapsed();

    let total_time = start_time.elapsed();

    // Print benchmark results
    println!("=== End-to-End Latency Benchmark ===");
    println!("Setup time: {:?}", setup_time);
    println!("Bundle submission time: {:?}", submission_time);
    println!("Auction time: {:?}", auction_time);
    println!("Block assembly time: {:?}", assembly_time);
    println!("Validator submission time: {:?}", validator_time);
    println!("Total end-to-end time: {:?}", total_time);

    // Assert reasonable performance (adjust thresholds as needed)
    assert!(total_time < Duration::from_millis(100), "End-to-end latency too high: {:?}", total_time);
}

#[tokio::test]
async fn test_block_assembler_transaction_limits() {
    let leader = Keypair::new();
    let assembler = BlockAssembler::new(leader.pubkey(), 5, 50_000); // Small limits for testing
    
    let bundles = vec![
        create_test_bundle(3000, 3), // 3 transactions
        create_test_bundle(2000, 2), // 2 transactions - should fit
        create_test_bundle(1000, 2), // 2 more transactions - should be rejected (would make 7 total)
    ];

    let template = assembler.create_block_template(1, Hash::new_unique());
    let block = assembler.assemble_block(template, bundles).await.expect("Failed to assemble block");

    // Should only include first two bundles due to transaction limit
    assert_eq!(block.bundles.len(), 2);
    assert_eq!(block.transactions.len(), 5); // Exactly at the limit
    assert_eq!(block.total_tips, 5000); // 3000 + 2000
}

#[tokio::test]
async fn test_auction_stats() {
    let mock_rpc = Box::new(MockSolanaRpcClient::new());
    let simulator = TransactionSimulator::new(mock_rpc);
    let mut auction = BundleAuction::new_with_simulator(42, simulator);

    // Add bundles with different tips
    let bundles = vec![
        create_test_bundle(1000, 1),
        create_test_bundle(2000, 1),
        create_test_bundle(3000, 1),
    ];

    for bundle in bundles {
        auction.add_bundle(bundle).await.expect("Failed to add bundle");
    }

    let stats = auction.get_auction_stats();
    
    assert_eq!(stats.slot, 42);
    assert_eq!(stats.total_bundles, 3);
    assert_eq!(stats.total_tip_value, 6000);
    assert_eq!(stats.highest_tip, 3000);
    assert_eq!(stats.avg_tip, 2000);
}

#[tokio::test]
async fn test_pool_tip_range_queries() {
    let pool = TransactionPool::new(100);
    
    // Add bundles with various tip amounts
    let tips = vec![500, 1000, 1500, 2000, 2500, 3000];
    for tip in tips {
        let bundle = create_test_bundle(tip, 1);
        pool.add_bundle(bundle).expect("Failed to add bundle");
    }

    // Query for bundles in specific tip ranges
    let mid_range_bundles = pool.get_bundles_by_tip_range(1000, 2500);
    assert_eq!(mid_range_bundles.len(), 4); // 1000, 1500, 2000, 2500

    let high_tip_bundles = pool.get_bundles_by_tip_range(2000, 5000);
    assert_eq!(high_tip_bundles.len(), 3); // 2000, 2500, 3000

    let exact_match = pool.get_bundles_by_tip_range(1500, 1500);
    assert_eq!(exact_match.len(), 1); // Only 1500
}

#[tokio::test]
async fn test_validator_client_failure_handling() {
    let mut validator_client = MockValidatorClient::new();
    validator_client.set_failure_mode(true);

    let bundle = create_test_bundle(1000, 1);
    let block = crate::block_assembler::Block {
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

    // Should fail when failure mode is enabled
    let result = validator_client.submit_block(block).await;
    assert!(result.is_err());

    // No blocks should be recorded when submission fails
    let submitted_blocks = validator_client.get_submitted_blocks();
    assert_eq!(submitted_blocks.len(), 0);
}

// Stress test with many bundles
#[tokio::test]
async fn test_high_volume_bundle_processing() {
    let pool = TransactionPool::new(10000);
    let mock_rpc = Box::new(MockSolanaRpcClient::new());
    let simulator = TransactionSimulator::new(mock_rpc);
    let mut auction = BundleAuction::new_with_simulator(1, simulator);
    
    const NUM_BUNDLES: usize = 1000;
    
    // Add many bundles to pool
    for i in 0..NUM_BUNDLES {
        let bundle = create_test_bundle((i as u64 + 1) * 100, 1); // Varying tips
        pool.add_bundle(bundle).expect("Failed to add bundle");
    }

    // Get all bundles and add to auction
    let bundles = pool.get_pending_bundles(NUM_BUNDLES);
    for bundle in bundles {
        auction.add_bundle(bundle).await.expect("Failed to add to auction");
    }

    // Select top 100 bundles
    let start_time = Instant::now();
    let winners = auction.select_winning_bundles(100);
    let selection_time = start_time.elapsed();

    println!("Selected {} winners from {} bundles in {:?}", 
             winners.len(), NUM_BUNDLES, selection_time);

    // Verify winners are sorted by tip (highest first)
    for i in 1..winners.len() {
        assert!(winners[i-1].tip_lamports >= winners[i].tip_lamports,
                "Winners not properly sorted by tip");
    }

    // Top winner should have the highest tip
    assert_eq!(winners[0].tip_lamports, NUM_BUNDLES as u64 * 100);
}
