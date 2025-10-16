#!/bin/bash

# Bundle simulation script for testing the permissionless block builder
# This script simulates multiple searchers submitting bundles with varying characteristics

set -e

# Configuration
API_BASE_URL="${API_BASE_URL:-http://localhost:3001}"
NUM_SEARCHERS="${NUM_SEARCHERS:-5}"
BUNDLES_PER_SEARCHER="${BUNDLES_PER_SEARCHER:-10}"
MIN_TIP="${MIN_TIP:-1000000}"    # 0.001 SOL in lamports
MAX_TIP="${MAX_TIP:-10000000}"   # 0.01 SOL in lamports
SIMULATION_DURATION="${SIMULATION_DURATION:-60}"  # seconds

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

# Generate a mock Solana transaction
generate_mock_transaction() {
    local from_pubkey="$1"
    local to_pubkey="$2"
    local amount="$3"
    
    cat << EOF
{
  "signatures": ["$(openssl rand -hex 32)$(openssl rand -hex 32)"],
  "message": {
    "accountKeys": [
      "$from_pubkey",
      "$to_pubkey",
      "11111111111111111111111111111112"
    ],
    "instructions": [
      {
        "programIdIndex": 2,
        "accounts": [0, 1],
        "data": "$(echo -n "$amount" | base64)"
      }
    ],
    "recentBlockhash": "$(openssl rand -hex 32)"
  }
}
EOF
}

# Generate a random Solana public key
generate_random_pubkey() {
    openssl rand -base64 32 | tr -d '=' | tr '+/' 'AB' | head -c 44
}

# Generate a bundle with random characteristics
generate_bundle() {
    local searcher_id="$1"
    local bundle_id="$2"
    
    local searcher_pubkey=$(generate_random_pubkey)
    local tip=$((MIN_TIP + RANDOM % (MAX_TIP - MIN_TIP)))
    local num_transactions=$((1 + RANDOM % 5))  # 1-5 transactions per bundle
    
    local transactions=""
    for ((i=0; i<num_transactions; i++)); do
        local from_pubkey=$(generate_random_pubkey)
        local to_pubkey=$(generate_random_pubkey)
        local amount=$((100000 + RANDOM % 900000))  # Random amount
        
        local tx=$(generate_mock_transaction "$from_pubkey" "$to_pubkey" "$amount")
        
        if [ $i -eq 0 ]; then
            transactions="$tx"
        else
            transactions="$transactions,$tx"
        fi
    done
    
    cat << EOF
{
  "transactions": [$transactions],
  "tipLamports": $tip,
  "searcherPubkey": "$searcher_pubkey"
}
EOF
}

# Submit a bundle to the API
submit_bundle() {
    local bundle_json="$1"
    local searcher_id="$2"
    local bundle_id="$3"
    
    local response=$(curl -s -w "%{http_code}" -X POST \
        -H "Content-Type: application/json" \
        -d "$bundle_json" \
        "$API_BASE_URL/api/bundles")
    
    local http_code="${response: -3}"
    local body="${response%???}"
    
    if [ "$http_code" -eq 200 ]; then
        local submitted_bundle_id=$(echo "$body" | grep -o '"bundleId":"[^"]*"' | cut -d'"' -f4)
        success "Searcher $searcher_id: Bundle $bundle_id submitted (ID: $submitted_bundle_id)"
        echo "$submitted_bundle_id" >> "/tmp/submitted_bundles_$searcher_id.txt"
    else
        error "Searcher $searcher_id: Bundle $bundle_id failed (HTTP $http_code): $body"
    fi
}

# Check bundle status
check_bundle_status() {
    local bundle_id="$1"
    local searcher_id="$2"
    
    local response=$(curl -s "$API_BASE_URL/api/bundles/$bundle_id")
    local status=$(echo "$response" | grep -o '"status":"[^"]*"' | cut -d'"' -f4)
    
    echo "Searcher $searcher_id: Bundle $bundle_id status: $status"
}

# Simulate a single searcher
simulate_searcher() {
    local searcher_id="$1"
    
    log "Starting searcher $searcher_id simulation..."
    
    # Clean up any previous run
    rm -f "/tmp/submitted_bundles_$searcher_id.txt"
    
    for ((bundle=1; bundle<=BUNDLES_PER_SEARCHER; bundle++)); do
        local bundle_json=$(generate_bundle "$searcher_id" "$bundle")
        submit_bundle "$bundle_json" "$searcher_id" "$bundle"
        
        # Random delay between submissions (0.1 to 2 seconds)
        local delay=$(echo "scale=1; (1 + $RANDOM % 20) / 10" | bc)
        sleep "$delay"
    done
    
    success "Searcher $searcher_id completed $BUNDLES_PER_SEARCHER bundle submissions"
}

# Monitor submitted bundles
monitor_bundles() {
    log "Starting bundle monitoring..."
    
    local check_interval=5
    local checks=$((SIMULATION_DURATION / check_interval))
    
    for ((i=1; i<=checks; i++)); do
        log "Bundle status check $i/$checks"
        
        for ((searcher=1; searcher<=NUM_SEARCHERS; searcher++)); do
            if [ -f "/tmp/submitted_bundles_$searcher.txt" ]; then
                while IFS= read -r bundle_id; do
                    [ -n "$bundle_id" ] && check_bundle_status "$bundle_id" "$searcher"
                done < "/tmp/submitted_bundles_$searcher.txt"
            fi
        done
        
        sleep $check_interval
    done
}

# Get system metrics
get_metrics() {
    log "Fetching system metrics..."
    
    local response=$(curl -s "$API_BASE_URL/api/status/metrics")
    echo "System Metrics:"
    echo "$response" | python3 -m json.tool 2>/dev/null || echo "$response"
}

# Cleanup function
cleanup() {
    log "Cleaning up temporary files..."
    for ((searcher=1; searcher<=NUM_SEARCHERS; searcher++)); do
        rm -f "/tmp/submitted_bundles_$searcher.txt"
    done
}

# Benchmark mode - measure latency and throughput
run_benchmark() {
    log "Running benchmark mode..."
    
    local start_time=$(date +%s.%N)
    local total_bundles=0
    local successful_bundles=0
    
    for ((searcher=1; searcher<=NUM_SEARCHERS; searcher++)); do
        for ((bundle=1; bundle<=BUNDLES_PER_SEARCHER; bundle++)); do
            local bundle_start=$(date +%s.%N)
            local bundle_json=$(generate_bundle "$searcher" "$bundle")
            
            local response=$(curl -s -w "%{http_code}" -X POST \
                -H "Content-Type: application/json" \
                -d "$bundle_json" \
                "$API_BASE_URL/api/bundles")
            
            local bundle_end=$(date +%s.%N)
            local bundle_latency=$(echo "$bundle_end - $bundle_start" | bc)
            
            local http_code="${response: -3}"
            total_bundles=$((total_bundles + 1))
            
            if [ "$http_code" -eq 200 ]; then
                successful_bundles=$((successful_bundles + 1))
            fi
            
            echo "Bundle $total_bundles: ${bundle_latency}s, HTTP $http_code"
        done
    done
    
    local end_time=$(date +%s.%N)
    local total_time=$(echo "$end_time - $start_time" | bc)
    local throughput=$(echo "scale=2; $total_bundles / $total_time" | bc)
    local success_rate=$(echo "scale=2; $successful_bundles * 100 / $total_bundles" | bc)
    
    echo ""
    echo "=== BENCHMARK RESULTS ==="
    echo "Total bundles: $total_bundles"
    echo "Successful bundles: $successful_bundles"
    echo "Success rate: $success_rate%"
    echo "Total time: ${total_time}s"
    echo "Throughput: $throughput bundles/sec"
    echo "========================="
}

# Stress test mode - high concurrency
run_stress_test() {
    log "Running stress test with high concurrency..."
    
    local pids=()
    
    # Start all searchers in parallel
    for ((searcher=1; searcher<=NUM_SEARCHERS; searcher++)); do
        simulate_searcher "$searcher" &
        pids+=($!)
    done
    
    # Start monitoring in background
    monitor_bundles &
    local monitor_pid=$!
    
    # Wait for all searchers to complete
    for pid in "${pids[@]}"; do
        wait "$pid"
    done
    
    # Stop monitoring
    kill "$monitor_pid" 2>/dev/null || true
    
    success "Stress test completed"
}

# Print usage
usage() {
    echo "Usage: $0 [simulate|benchmark|stress|monitor]"
    echo ""
    echo "Modes:"
    echo "  simulate  - Run normal simulation with multiple searchers"
    echo "  benchmark - Measure latency and throughput"
    echo "  stress    - High concurrency stress test"
    echo "  monitor   - Only monitor existing bundles"
    echo ""
    echo "Environment variables:"
    echo "  API_BASE_URL=$API_BASE_URL"
    echo "  NUM_SEARCHERS=$NUM_SEARCHERS"
    echo "  BUNDLES_PER_SEARCHER=$BUNDLES_PER_SEARCHER"
    echo "  MIN_TIP=$MIN_TIP"
    echo "  MAX_TIP=$MAX_TIP"
    echo "  SIMULATION_DURATION=$SIMULATION_DURATION"
}

# Check dependencies
check_deps() {
    if ! command -v curl &> /dev/null; then
        error "curl is required but not installed"
        exit 1
    fi
    
    if ! command -v bc &> /dev/null; then
        error "bc is required but not installed"
        exit 1
    fi
    
    if ! command -v openssl &> /dev/null; then
        error "openssl is required but not installed"
        exit 1
    fi
}

# Main execution
main() {
    local mode="${1:-simulate}"
    
    check_deps
    
    log "Bundle Simulation Starting"
    log "API Base URL: $API_BASE_URL"
    log "Searchers: $NUM_SEARCHERS"
    log "Bundles per searcher: $BUNDLES_PER_SEARCHER"
    log "Tip range: $MIN_TIP - $MAX_TIP lamports"
    log "Mode: $mode"
    echo ""
    
    # Set up cleanup trap
    trap cleanup EXIT
    
    case "$mode" in
        "simulate")
            run_stress_test
            get_metrics
            ;;
        "benchmark")
            run_benchmark
            get_metrics
            ;;
        "stress")
            run_stress_test
            ;;
        "monitor")
            monitor_bundles
            ;;
        *)
            usage
            exit 1
            ;;
    esac
    
    success "Simulation completed successfully!"
}

# Run main function with all arguments
main "$@"
