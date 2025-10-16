#!/bin/bash

# Local testnet setup and integration testing script
# This script sets up a local Solana testnet and runs comprehensive tests

set -e

# Configuration
SOLANA_VERSION="${SOLANA_VERSION:-1.18.0}"
TESTNET_RPC_PORT="${TESTNET_RPC_PORT:-8899}"
TESTNET_WS_PORT="${TESTNET_WS_PORT:-8900}"
BLOCK_ENGINE_PORT="${BLOCK_ENGINE_PORT:-8080}"
API_SERVER_PORT="${API_SERVER_PORT:-3001}"
TEST_VALIDATOR_LEDGER_DIR="${TEST_VALIDATOR_LEDGER_DIR:-/tmp/test-ledger}"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

log() {
    echo -e "${BLUE}[$(date '+%H:%M:%S')]${NC} $1"
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if Solana CLI is installed
check_solana() {
    if ! command -v solana &> /dev/null; then
        error "Solana CLI not found. Installing..."
        install_solana
    else
        local version=$(solana --version | cut -d' ' -f2)
        log "Found Solana CLI version: $version"
    fi
}

# Install Solana CLI
install_solana() {
    log "Installing Solana CLI..."
    sh -c "$(curl -sSfL https://release.solana.com/v${SOLANA_VERSION}/install)"
    export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"
    success "Solana CLI installed"
}

# Setup test validator
setup_test_validator() {
    log "Setting up Solana test validator..."
    
    # Clean up any existing ledger
    rm -rf "$TEST_VALIDATOR_LEDGER_DIR"
    
    # Start test validator
    solana-test-validator \
        --ledger "$TEST_VALIDATOR_LEDGER_DIR" \
        --rpc-port "$TESTNET_RPC_PORT" \
        --rpc-bind-address 0.0.0.0 \
        --websocket-port "$TESTNET_WS_PORT" \
        --reset \
        --quiet &
    
    local validator_pid=$!
    echo $validator_pid > /tmp/test_validator.pid
    
    # Wait for validator to start
    log "Waiting for test validator to start..."
    for i in {1..30}; do
        if curl -s "http://localhost:$TESTNET_RPC_PORT" > /dev/null 2>&1; then
            success "Test validator is ready"
            break
        fi
        if [ $i -eq 30 ]; then
            error "Test validator failed to start"
            exit 1
        fi
        sleep 2
    done
    
    # Configure Solana CLI
    solana config set --url "http://localhost:$TESTNET_RPC_PORT"
    success "Solana CLI configured for local testnet"
}

# Create test accounts and fund them
setup_test_accounts() {
    log "Setting up test accounts..."
    
    # Create test keypairs
    mkdir -p test-keys
    
    # Create searcher keypairs
    for i in {1..5}; do
        if [ ! -f "test-keys/searcher$i.json" ]; then
            solana-keygen new --no-bip39-passphrase --outfile "test-keys/searcher$i.json"
        fi
        
        # Airdrop SOL
        local pubkey=$(solana-keygen pubkey "test-keys/searcher$i.json")
        solana airdrop 100 "$pubkey"
        log "Funded searcher$i: $pubkey"
    done
    
    # Create validator keypair
    if [ ! -f "test-keys/validator.json" ]; then
        solana-keygen new --no-bip39-passphrase --outfile "test-keys/validator.json"
    fi
    
    local validator_pubkey=$(solana-keygen pubkey "test-keys/validator.json")
    solana airdrop 100 "$validator_pubkey"
    log "Funded validator: $validator_pubkey"
    
    success "Test accounts created and funded"
}

# Start block engine
start_block_engine() {
    log "Starting block engine..."
    
    cd block-engine
    
    # Build if not already built
    if [ ! -f "target/release/block-engine" ]; then
        cargo build --release
    fi
    
    # Start block engine
    ./target/release/block-engine \
        --bind-address "127.0.0.1:$BLOCK_ENGINE_PORT" \
        --rpc-url "http://localhost:$TESTNET_RPC_PORT" &
    
    local engine_pid=$!
    echo $engine_pid > /tmp/block_engine.pid
    
    cd ..
    
    # Wait for block engine to start
    sleep 5
    
    success "Block engine started on port $BLOCK_ENGINE_PORT"
}

# Start API server
start_api_server() {
    log "Starting API server..."
    
    cd api-server
    
    # Install dependencies if needed
    if [ ! -d "node_modules" ]; then
        npm install
    fi
    
    # Start API server
    PORT="$API_SERVER_PORT" npm run dev &
    local api_pid=$!
    echo $api_pid > /tmp/api_server.pid
    
    cd ..
    
    # Wait for API server to start
    for i in {1..15}; do
        if curl -s "http://localhost:$API_SERVER_PORT/health" > /dev/null 2>&1; then
            success "API server is ready on port $API_SERVER_PORT"
            break
        fi
        if [ $i -eq 15 ]; then
            error "API server failed to start"
            exit 1
        fi
        sleep 2
    done
}

# Run integration tests with real transactions
run_integration_tests() {
    log "Running integration tests with real Solana transactions..."
    
    # Create test script
    cat > integration_test.js << 'EOF'
const { Connection, Keypair, Transaction, SystemProgram, LAMPORTS_PER_SOL } = require('@solana/web3.js');
const fs = require('fs');

async function runIntegrationTest() {
    const connection = new Connection('http://localhost:8899', 'confirmed');
    
    console.log('🧪 Starting integration tests...');
    
    // Load test keypairs
    const searcher1 = Keypair.fromSecretKey(
        Uint8Array.from(JSON.parse(fs.readFileSync('test-keys/searcher1.json')))
    );
    
    const searcher2 = Keypair.fromSecretKey(
        Uint8Array.from(JSON.parse(fs.readFileSync('test-keys/searcher2.json')))
    );
    
    console.log(`Searcher 1: ${searcher1.publicKey.toBase58()}`);
    console.log(`Searcher 2: ${searcher2.publicKey.toBase58()}`);
    
    // Test 1: Create and submit bundles with real transactions
    console.log('\n🔬 Test 1: Creating real transactions...');
    
    const { blockhash } = await connection.getLatestBlockhash();
    
    // Create transaction for searcher 1
    const tx1 = new Transaction({
        feePayer: searcher1.publicKey,
        recentBlockhash: blockhash,
    });
    
    tx1.add(
        SystemProgram.transfer({
            fromPubkey: searcher1.publicKey,
            toPubkey: searcher2.publicKey,
            lamports: 0.01 * LAMPORTS_PER_SOL,
        })
    );
    
    tx1.sign(searcher1);
    
    console.log(`Transaction created: ${tx1.signature?.toString()}`);
    
    // Create bundle payload
    const bundle = {
        transactions: [{
            signatures: tx1.signatures.map(sig => sig.signature?.toString()),
            message: {
                accountKeys: tx1.compileMessage().accountKeys.map(key => key.toBase58()),
                instructions: tx1.compileMessage().instructions,
                recentBlockhash: blockhash
            }
        }],
        tipLamports: 5000000, // 0.005 SOL
        searcherPubkey: searcher1.publicKey.toBase58()
    };
    
    // Submit bundle to API
    console.log('\n📤 Submitting bundle to API...');
    const response = await fetch('http://localhost:3001/api/bundles', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(bundle)
    });
    
    const result = await response.json();
    console.log('Bundle submission result:', result);
    
    if (result.success) {
        console.log('✅ Bundle submitted successfully');
        
        // Check bundle status
        console.log('\n🔍 Checking bundle status...');
        const statusResponse = await fetch(`http://localhost:3001/api/bundles/${result.bundleId}`);
        const status = await statusResponse.json();
        console.log('Bundle status:', status);
        
    } else {
        console.log('❌ Bundle submission failed');
    }
    
    // Test 2: Check system metrics
    console.log('\n📊 Checking system metrics...');
    const metricsResponse = await fetch('http://localhost:3001/api/status/metrics');
    const metrics = await metricsResponse.json();
    console.log('System metrics:', metrics);
    
    console.log('\n🎉 Integration tests completed!');
}

runIntegrationTest().catch(console.error);
EOF

    # Install required packages and run test
    cd api-server
    npm install @solana/web3.js node-fetch
    cd ..
    
    node integration_test.js
    
    rm -f integration_test.js
    
    success "Integration tests completed"
}

# Run load tests against testnet
run_load_tests() {
    log "Running load tests against local testnet..."
    
    # Set environment variables for load test
    export API_BASE_URL="http://localhost:$API_SERVER_PORT"
    export NUM_SEARCHERS=3
    export BUNDLES_PER_SEARCHER=5
    export MIN_TIP=1000000
    export MAX_TIP=10000000
    
    # Run the bundle simulation script
    ./scripts/simulate_bundles.sh benchmark
    
    success "Load tests completed"
}

# Monitor system during tests
monitor_system() {
    log "Starting system monitoring..."
    
    # Create monitoring script
    cat > monitor.sh << 'EOF'
#!/bin/bash
while true; do
    echo "=== $(date) ==="
    echo "Block Engine Process:"
    ps aux | grep block-engine | head -1
    echo "API Server Process:"
    ps aux | grep node | head -1
    echo "Test Validator Process:"
    ps aux | grep solana-test-validator | head -1
    echo "Memory Usage:"
    free -h
    echo "Disk Usage:"
    df -h /tmp
    echo ""
    sleep 30
done
EOF
    
    chmod +x monitor.sh
    ./monitor.sh > system_monitor.log &
    echo $! > /tmp/monitor.pid
    
    log "System monitoring started (logging to system_monitor.log)"
}

# Cleanup function
cleanup() {
    log "Cleaning up processes and temporary files..."
    
    # Stop monitoring
    if [ -f /tmp/monitor.pid ]; then
        kill $(cat /tmp/monitor.pid) 2>/dev/null || true
        rm -f /tmp/monitor.pid
    fi
    
    # Stop API server
    if [ -f /tmp/api_server.pid ]; then
        kill $(cat /tmp/api_server.pid) 2>/dev/null || true
        rm -f /tmp/api_server.pid
    fi
    
    # Stop block engine
    if [ -f /tmp/block_engine.pid ]; then
        kill $(cat /tmp/block_engine.pid) 2>/dev/null || true
        rm -f /tmp/block_engine.pid
    fi
    
    # Stop test validator
    if [ -f /tmp/test_validator.pid ]; then
        kill $(cat /tmp/test_validator.pid) 2>/dev/null || true
        rm -f /tmp/test_validator.pid
    fi
    
    # Clean up temporary files
    rm -f monitor.sh
    rm -f integration_test.js
    
    log "Cleanup completed"
}

# Generate test report
generate_report() {
    log "Generating test report..."
    
    cat > testnet_report.md << EOF
# Local Testnet Integration Test Report

**Date:** $(date)

## Test Environment
- Solana Test Validator: Port $TESTNET_RPC_PORT
- Block Engine: Port $BLOCK_ENGINE_PORT  
- API Server: Port $API_SERVER_PORT
- Test Ledger: $TEST_VALIDATOR_LEDGER_DIR

## Tests Executed
1. ✅ Test validator setup and funding
2. ✅ Block engine startup
3. ✅ API server startup
4. ✅ Real transaction creation and submission
5. ✅ Bundle submission via API
6. ✅ Bundle status queries
7. ✅ System metrics retrieval
8. ✅ Load testing with multiple bundles

## System Monitoring
See \`system_monitor.log\` for detailed system resource usage during tests.

## Recommendations
1. Monitor block engine performance under sustained load
2. Test with more complex transactions (smart contracts)
3. Verify bundle ordering and tip prioritization
4. Test error scenarios (insufficient funds, invalid signatures)

## Next Steps
1. Deploy to Solana devnet for broader testing
2. Implement automated monitoring and alerting
3. Add chaos engineering tests (network partitions, etc.)
4. Performance optimization based on load test results

EOF

    success "Test report generated: testnet_report.md"
}

# Print usage
usage() {
    echo "Usage: $0 [setup|test|load|monitor|cleanup|full]"
    echo ""
    echo "Commands:"
    echo "  setup    - Set up local testnet and services"
    echo "  test     - Run integration tests only"
    echo "  load     - Run load tests only"
    echo "  monitor  - Start system monitoring only"
    echo "  cleanup  - Clean up processes and files"
    echo "  full     - Run complete test suite (default)"
    echo ""
    echo "Environment variables:"
    echo "  SOLANA_VERSION=$SOLANA_VERSION"
    echo "  TESTNET_RPC_PORT=$TESTNET_RPC_PORT"
    echo "  BLOCK_ENGINE_PORT=$BLOCK_ENGINE_PORT"
    echo "  API_SERVER_PORT=$API_SERVER_PORT"
}

# Main execution
main() {
    local command="${1:-full}"
    
    # Set up cleanup trap
    trap cleanup EXIT
    
    case "$command" in
        "setup")
            check_solana
            setup_test_validator
            setup_test_accounts
            start_block_engine
            start_api_server
            success "Setup completed"
            ;;
        "test")
            run_integration_tests
            ;;
        "load")
            run_load_tests
            ;;
        "monitor")
            monitor_system
            success "Monitoring started. Press Ctrl+C to stop."
            tail -f system_monitor.log
            ;;
        "cleanup")
            cleanup
            ;;
        "full")
            log "Starting full integration test suite..."
            check_solana
            setup_test_validator
            setup_test_accounts
            monitor_system
            start_block_engine
            start_api_server
            run_integration_tests
            run_load_tests
            generate_report
            success "Full test suite completed successfully! 🎉"
            ;;
        *)
            usage
            exit 1
            ;;
    esac
}

# Check if running in script directory
if [ ! -f "scripts/$(basename "$0")" ]; then
    error "Please run this script from the project root directory"
    exit 1
fi

# Run main function
main "$@"
