// Import our auction modules
mod auction;
mod bundle;
mod simulator;
mod block_assembler;
mod validator;

use auction::{simulate_auction_with_bundles, simulate_auction_window};
use bundle::Bundle;
use block_assembler::{assemble_block_with_params};
use validator::{MockValidator, ValidatorNetwork, BlockSubmissionResult};
use solana_sdk::{hash::Hash, pubkey::Pubkey, transaction::Transaction, instruction::Instruction, message::Message, signature::Signature};
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use tracing::{info, Level};
use tracing_subscriber;
use serde_json;

/// Example demonstrating the 200ms auction window functionality
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("🚀 Starting complete auction, block assembly, and validator demo");

    // Run both examples with validator submission
    demo_auction_with_block_assembly_and_validation().await?;
    demo_real_time_auction_with_validator_network().await?;

    info!("✅ Complete demo finished");
    Ok(())
}

/// Demo with pre-collected bundles, block assembly, and validator submission
async fn demo_auction_with_block_assembly_and_validation() -> anyhow::Result<()> {
    info!("📦 Demo 1: Auction + Block Assembly + Validator Submission");

    // Create sample bundles with different priority fees
    let sample_bundles = create_sample_bundles();
    
    // Simulate auction window with these bundles
    let window_id = 12345;
    let max_bundles_for_block = 3;
    
    let winners = simulate_auction_with_bundles(
        window_id,
        sample_bundles.clone(),
        max_bundles_for_block,
    )?;
    
    info!("🏆 Auction complete: {} winners selected", winners.len());

    // Assemble block with the winning bundles
    let slot = 12345;
    let parent_hash = Hash::default();
    let leader_pubkey = Pubkey::default();

    let (block, summary) = assemble_block_with_params(
        winners,
        slot,
        parent_hash,
        leader_pubkey,
    )?;

    // Print JSON summary
    let summary_json = serde_json::to_string_pretty(&summary)?;
    info!("📋 Block Summary JSON:\n{}", summary_json);

    info!("⛓️ Block assembled with {} transactions", block.transactions.len());

    // Submit to mock validator
    let validator = MockValidator::new();
    info!("🔍 Submitting block to validator for verification...");
    
    match validator.submit_block(block).await? {
        BlockSubmissionResult::Accepted { signature } => {
            info!("✅ Demo 1 SUCCESS: Block accepted by validator with signature {}", signature);
        }
        BlockSubmissionResult::Rejected { reason } => {
            info!("❌ Demo 1 FAILURE: Block rejected by validator: {}", reason);
        }
    }

    // Print validator stats
    let stats = validator.get_stats();
    info!("📊 Validator stats: {} accepted, {} rejected", stats.blocks_accepted, stats.blocks_rejected);

    Ok(())
}

/// Demo with real-time bundle collection, block assembly, and validator network
async fn demo_real_time_auction_with_validator_network() -> anyhow::Result<()> {
    info!("⏱️ Demo 2: Real-time Auction + Block Assembly + Validator Network");

    let (bundle_sender, bundle_receiver) = mpsc::channel::<Bundle>(100);
    let window_id = 12346;
    let max_bundles_for_block = 5;

    // Spawn a task to simulate bundles arriving over time
    let sender_handle = tokio::spawn(async move {
        simulate_bundle_arrivals(bundle_sender).await
    });

    // Run the auction window
    let winners = simulate_auction_window(
        window_id,
        bundle_receiver,
        max_bundles_for_block,
    ).await?;

    // Wait for the sender to complete
    sender_handle.await?;

    info!("🏆 Auction complete: {} winners selected", winners.len());

    // Assemble block with the winning bundles
    let slot = 12346;
    let parent_hash = Hash::default();
    let leader_pubkey = Pubkey::default();

    let (block, summary) = assemble_block_with_params(
        winners,
        slot,
        parent_hash,
        leader_pubkey,
    )?;

    // Print JSON summary
    let summary_json = serde_json::to_string_pretty(&summary)?;
    info!("📋 Block Summary JSON:\n{}", summary_json);

    info!("⛓️ Block assembled with {} transactions", block.transactions.len());

    // Submit to validator network
    let network = ValidatorNetwork::new(5); // 5 validators
    info!("🌐 Submitting block to validator network (5 validators)...");
    
    let results = network.submit_block_to_network(block).await;
    
    // Analyze results
    let accepted_count = results.iter().filter(|(_, result)| {
        matches!(result, BlockSubmissionResult::Accepted { .. })
    }).count();
    
    let rejected_count = results.len() - accepted_count;
    
    info!("📊 Network results: {} accepted, {} rejected", accepted_count, rejected_count);
    
    // Show individual validator results
    for (validator_id, result) in results {
        match result {
            BlockSubmissionResult::Accepted { signature } => {
                info!("✅ {} ACCEPTED block with signature {}", validator_id, signature);
            }
            BlockSubmissionResult::Rejected { reason } => {
                info!("❌ {} REJECTED block: {}", validator_id, reason);
            }
        }
    }

    // Print network stats
    let network_stats = network.get_network_stats();
    for stats in network_stats {
        info!(
            "📈 {}: {} accepted, {} rejected, {:.1}% acceptance rate",
            stats.validator_id,
            stats.blocks_accepted,
            stats.blocks_rejected,
            stats.acceptance_rate * 100.0
        );
    }

    info!("✅ Demo 2 complete: Network validation finished");
    Ok(())
}

/// Simulate bundles arriving over time during the auction window
async fn simulate_bundle_arrivals(sender: mpsc::Sender<Bundle>) {
    info!("📡 Simulating bundle arrivals over 200ms window");

    // Send bundles at different times with varying priority fees
    let bundle_scenarios = vec![
        (0, 1000000, "searcher_alice"),     // Immediate: 1 SOL tip
        (50, 500000, "searcher_bob"),       // 50ms: 0.5 SOL tip  
        (100, 2000000, "searcher_charlie"), // 100ms: 2 SOL tip (highest)
        (120, 750000, "searcher_dave"),     // 120ms: 0.75 SOL tip
        (150, 1500000, "searcher_eve"),     // 150ms: 1.5 SOL tip
        (180, 300000, "searcher_frank"),    // 180ms: 0.3 SOL tip
        (220, 5000000, "searcher_late"),    // 220ms: 5 SOL tip (too late!)
    ];

    for (delay_ms, tip_lamports, searcher) in bundle_scenarios {
        if delay_ms > 0 {
            sleep(Duration::from_millis(delay_ms)).await;
        }

        let bundle = Bundle::new(
            create_mock_transactions(1), // 1 transaction per bundle
            tip_lamports,
            searcher.to_string(),
        );

        info!(
            "📤 Sending bundle {} from {} with tip {} lamports at {}ms",
            bundle.id, searcher, tip_lamports, delay_ms
        );

        if let Err(e) = sender.send(bundle).await {
            tracing::warn!("Failed to send bundle: {}", e);
            break;
        }
    }

    info!("📡 Finished sending bundles");
}

/// Create sample bundles for testing with mock transactions
fn create_sample_bundles() -> Vec<Bundle> {
    vec![
        Bundle::new(
            create_mock_transactions(2), // 2 transactions
            1500000,
            "searcher_high_roller".to_string(),
        ),
        Bundle::new(
            create_mock_transactions(1), // 1 transaction
            500000,
            "searcher_budget".to_string(),
        ),
        Bundle::new(
            create_mock_transactions(3), // 3 transactions
            2500000,
            "searcher_whale".to_string(),
        ),
        Bundle::new(
            create_mock_transactions(1), // 1 transaction
            750000,
            "searcher_medium".to_string(),
        ),
        Bundle::new(
            create_mock_transactions(2), // 2 transactions
            1000000,
            "searcher_standard".to_string(),
        ),
        Bundle::new(
            create_mock_transactions(1), // 1 transaction
            100000,
            "searcher_lowball".to_string(),
        ),
    ]
}

/// Create mock transactions for demo purposes
fn create_mock_transactions(count: usize) -> Vec<Transaction> {
    let mut transactions = Vec::new();
    
    for i in 0..count {
        // Create a simple mock transaction
        let instruction = Instruction::new_with_bytes(
            Pubkey::new_unique(), // Random program ID
            &[i as u8; 32],       // Mock instruction data
            vec![], // No accounts for demo
        );
        
        let message = Message::new(
            &[instruction],
            Some(&Pubkey::new_unique()), // Mock fee payer
        );
        
        let transaction = Transaction {
            signatures: vec![Signature::default()], // Mock signature
            message,
        };
        
        transactions.push(transaction);
    }
    
    transactions
}
