use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use block_engine::{
    auction::BundleAuction,
    block_assembler::{BlockAssembler, MockValidatorClient},
    bundle::Bundle,
    transaction_pool::TransactionPool,
    simulator::{MockSolanaRpcClient, TransactionSimulator},
};
use solana_sdk::{
    hash::Hash,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};
use std::time::Duration;
use tokio::runtime::Runtime;

fn create_test_bundle(tip: u64, tx_count: usize) -> Bundle {
    let keypair = Keypair::new();
    let mut transactions = Vec::new();
    
    for _ in 0..tx_count {
        let instruction = system_instruction::transfer(&keypair.pubkey(), &keypair.pubkey(), 100);
        let mut transaction = Transaction::new_with_payer(&[instruction], Some(&keypair.pubkey()));
        transaction.sign(&[&keypair], Hash::new_unique());
        transactions.push(transaction);
    }
    
    Bundle::new(transactions, tip, keypair.pubkey().to_string())
}

fn benchmark_transaction_pool(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("pool_add_bundle", |b| {
        b.iter(|| {
            let pool = TransactionPool::new(10000);
            let bundle = create_test_bundle(black_box(1000), black_box(1));
            pool.add_bundle(bundle).unwrap();
        });
    });

    let mut group = c.benchmark_group("pool_scale");
    for size in [100, 500, 1000, 5000].iter() {
        group.bench_with_input(BenchmarkId::new("add_bundles", size), size, |b, &size| {
            b.iter(|| {
                let pool = TransactionPool::new(size);
                for i in 0..size {
                    let bundle = create_test_bundle(black_box(i as u64), black_box(1));
                    pool.add_bundle(bundle).unwrap();
                }
            });
        });
    }
    group.finish();
}

fn benchmark_auction_engine(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("auction_select_winners", |b| {
        b.to_async(&rt).iter(|| async {
            let mock_rpc = Box::new(MockSolanaRpcClient::new());
            let simulator = TransactionSimulator::new(mock_rpc);
            let mut auction = BundleAuction::new_with_simulator(1, simulator);
            
            // Add 1000 bundles
            for i in 0..1000 {
                let bundle = create_test_bundle(i, 1);
                auction.add_bundle(bundle).await.unwrap();
            }
            
            let _winners = auction.select_winning_bundles(black_box(10));
        });
    });

    let mut group = c.benchmark_group("auction_scale");
    for bundle_count in [100, 500, 1000, 2000].iter() {
        group.bench_with_input(
            BenchmarkId::new("select_winners", bundle_count), 
            bundle_count, 
            |b, &bundle_count| {
                b.to_async(&rt).iter(|| async {
                    let mock_rpc = Box::new(MockSolanaRpcClient::new());
                    let simulator = TransactionSimulator::new(mock_rpc);
                    let mut auction = BundleAuction::new_with_simulator(1, simulator);
                    
                    for i in 0..bundle_count {
                        let bundle = create_test_bundle(i as u64, 1);
                        auction.add_bundle(bundle).await.unwrap();
                    }
                    
                    let _winners = auction.select_winning_bundles(10);
                });
            }
        );
    }
    group.finish();
}

fn benchmark_simulation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("simulate_bundle", |b| {
        b.to_async(&rt).iter(|| async {
            let mock_rpc = Box::new(MockSolanaRpcClient::new());
            let simulator = TransactionSimulator::new(mock_rpc);
            let bundle = create_test_bundle(black_box(1000), black_box(2));
            
            let _results = simulator.simulate_bundle(&bundle).await.unwrap();
        });
    });

    let mut group = c.benchmark_group("simulation_scale");
    for tx_count in [1, 2, 3, 4, 5].iter() {
        group.bench_with_input(
            BenchmarkId::new("simulate_transactions", tx_count), 
            tx_count, 
            |b, &tx_count| {
                b.to_async(&rt).iter(|| async {
                    let mock_rpc = Box::new(MockSolanaRpcClient::new());
                    let simulator = TransactionSimulator::new(mock_rpc);
                    let bundle = create_test_bundle(1000, tx_count);
                    
                    let _results = simulator.simulate_bundle(&bundle).await.unwrap();
                });
            }
        );
    }
    group.finish();
}

fn benchmark_block_assembly(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("assemble_block", |b| {
        b.to_async(&rt).iter(|| async {
            let leader = Keypair::new();
            let assembler = BlockAssembler::new(leader.pubkey(), 100, 500_000);
            let template = assembler.create_block_template(1, Hash::new_unique());
            
            let bundles = (0..10)
                .map(|i| create_test_bundle(i * 1000, 2))
                .collect();
            
            let _block = assembler.assemble_block(template, bundles).await.unwrap();
        });
    });

    let mut group = c.benchmark_group("block_assembly_scale");
    for bundle_count in [5, 10, 20, 50].iter() {
        group.bench_with_input(
            BenchmarkId::new("assemble_bundles", bundle_count), 
            bundle_count, 
            |b, &bundle_count| {
                b.to_async(&rt).iter(|| async {
                    let leader = Keypair::new();
                    let assembler = BlockAssembler::new(leader.pubkey(), 200, 1_000_000);
                    let template = assembler.create_block_template(1, Hash::new_unique());
                    
                    let bundles = (0..bundle_count)
                        .map(|i| create_test_bundle(i as u64 * 1000, 1))
                        .collect();
                    
                    let _block = assembler.assemble_block(template, bundles).await.unwrap();
                });
            }
        );
    }
    group.finish();
}

fn benchmark_end_to_end(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("end_to_end_pipeline", |b| {
        b.to_async(&rt).iter(|| async {
            // Setup
            let pool = TransactionPool::new(1000);
            let mock_rpc = Box::new(MockSolanaRpcClient::new());
            let simulator = TransactionSimulator::new(mock_rpc);
            let mut auction = BundleAuction::new_with_simulator(1, simulator);
            let leader = Keypair::new();
            let assembler = BlockAssembler::new(leader.pubkey(), 50, 500_000);
            let validator_client = MockValidatorClient::new();

            // Add bundles to pool
            for i in 0..black_box(20) {
                let bundle = create_test_bundle(i * 1000, 1);
                pool.add_bundle(bundle).unwrap();
            }

            // Get bundles from pool and add to auction
            let pending_bundles = pool.get_pending_bundles(20);
            for bundle in pending_bundles {
                auction.add_bundle(bundle).await.unwrap();
            }

            // Select winners
            let winners = auction.select_winning_bundles(10);

            // Assemble block
            let template = assembler.create_block_template(1, Hash::new_unique());
            let block = assembler.assemble_block(template, winners).await.unwrap();

            // Submit to validator
            let _signature = validator_client.submit_block(block).await.unwrap();
        });
    });
}

criterion_group!(
    benches,
    benchmark_transaction_pool,
    benchmark_auction_engine,
    benchmark_simulation,
    benchmark_block_assembly,
    benchmark_end_to_end
);
criterion_main!(benches);
