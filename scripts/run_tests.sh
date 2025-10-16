#!/bin/bash

# Comprehensive test suite runner for Permissionless Block Builder
# This script runs all tests and generates coverage reports

set -e

echo "🚀 Starting Comprehensive Test Suite for Permissionless Block Builder"
echo "================================================================"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if required tools are installed
check_dependencies() {
    print_status "Checking dependencies..."
    
    if ! command -v cargo &> /dev/null; then
        print_error "Cargo not found. Please install Rust."
        exit 1
    fi
    
    if ! command -v node &> /dev/null; then
        print_error "Node.js not found. Please install Node.js."
        exit 1
    fi
    
    if ! command -v npm &> /dev/null; then
        print_error "npm not found. Please install npm."
        exit 1
    fi
    
    print_success "All dependencies found"
}

# Build Rust components
build_rust_components() {
    print_status "Building Rust components..."
    
    cd block-engine
    cargo build --release
    if [ $? -eq 0 ]; then
        print_success "Rust components built successfully"
    else
        print_error "Failed to build Rust components"
        exit 1
    fi
    cd ..
}

# Install Node.js dependencies
install_node_dependencies() {
    print_status "Installing Node.js dependencies..."
    
    cd api-server
    npm install
    if [ $? -eq 0 ]; then
        print_success "Node.js dependencies installed"
    else
        print_error "Failed to install Node.js dependencies"
        exit 1
    fi
    cd ..
}

# Run Rust tests
run_rust_tests() {
    print_status "Running Rust tests..."
    
    cd block-engine
    
    # Run unit tests
    print_status "Running unit tests..."
    cargo test --lib -- --nocapture
    
    # Run integration tests
    print_status "Running integration tests..."
    cargo test integration_tests -- --nocapture --test-threads=1
    
    # Run doctests
    print_status "Running doctests..."
    cargo test --doc
    
    # Generate test coverage (requires cargo-tarpaulin)
    if command -v cargo-tarpaulin &> /dev/null; then
        print_status "Generating Rust code coverage..."
        cargo tarpaulin --out Html --output-dir ../coverage/rust
        print_success "Rust coverage report generated in coverage/rust/"
    else
        print_warning "cargo-tarpaulin not found. Install with: cargo install cargo-tarpaulin"
    fi
    
    cd ..
    print_success "Rust tests completed"
}

# Run Node.js tests
run_node_tests() {
    print_status "Running Node.js tests..."
    
    cd api-server
    
    # Run tests with coverage
    npm run test:coverage
    
    if [ $? -eq 0 ]; then
        print_success "Node.js tests completed"
    else
        print_error "Node.js tests failed"
        cd ..
        exit 1
    fi
    
    cd ..
}

# Run performance benchmarks
run_benchmarks() {
    print_status "Running performance benchmarks..."
    
    cd block-engine
    
    # Run benchmark tests
    cargo test test_end_to_end_latency_benchmark -- --nocapture --ignored
    cargo test test_high_volume_bundle_processing -- --nocapture --ignored
    
    cd ..
    print_success "Performance benchmarks completed"
}

# Start test services
start_test_services() {
    print_status "Starting test services..."
    
    # Start Redis for testing (if available)
    if command -v redis-server &> /dev/null; then
        redis-server --daemonize yes --port 6380 --dir /tmp
        print_success "Redis test server started on port 6380"
    else
        print_warning "Redis not found. Some integration tests may fail."
    fi
}

# Stop test services
stop_test_services() {
    print_status "Stopping test services..."
    
    # Stop Redis test server
    if command -v redis-cli &> /dev/null; then
        redis-cli -p 6380 shutdown
        print_success "Redis test server stopped"
    fi
}

# Generate comprehensive report
generate_report() {
    print_status "Generating comprehensive test report..."
    
    mkdir -p test-reports
    
    cat > test-reports/summary.md << EOF
# Test Report - $(date)

## Test Summary

### Rust Tests
- Unit tests: ✅ Passed
- Integration tests: ✅ Passed
- Documentation tests: ✅ Passed

### Node.js Tests
- API endpoint tests: ✅ Passed
- Integration tests: ✅ Passed

### Performance Benchmarks
- End-to-end latency: ✅ Measured
- High volume processing: ✅ Measured

## Coverage Reports
- Rust coverage: Available in \`coverage/rust/\`
- Node.js coverage: Available in \`api-server/coverage/\`

## Recommendations
1. Monitor end-to-end latency to ensure it stays below 100ms
2. Ensure bundle validation catches all edge cases
3. Test with larger transaction volumes periodically
4. Consider adding more chaos engineering tests

EOF

    print_success "Test report generated in test-reports/summary.md"
}

# Run load tests
run_load_tests() {
    print_status "Running load tests..."
    
    # Create a simple load test script
    cat > load_test.js << 'EOF'
const http = require('http');

const NUM_REQUESTS = 100;
const CONCURRENT_REQUESTS = 10;

function makeRequest() {
    return new Promise((resolve) => {
        const startTime = Date.now();
        const req = http.request({
            hostname: 'localhost',
            port: 3001,
            path: '/api/status/metrics',
            method: 'GET'
        }, (res) => {
            let data = '';
            res.on('data', chunk => data += chunk);
            res.on('end', () => {
                resolve({
                    status: res.statusCode,
                    latency: Date.now() - startTime,
                    success: res.statusCode === 200
                });
            });
        });
        
        req.on('error', () => {
            resolve({
                status: 0,
                latency: Date.now() - startTime,
                success: false
            });
        });
        
        req.end();
    });
}

async function runLoadTest() {
    console.log(`Starting load test: ${NUM_REQUESTS} requests with ${CONCURRENT_REQUESTS} concurrent`);
    
    const results = [];
    for (let i = 0; i < NUM_REQUESTS; i += CONCURRENT_REQUESTS) {
        const batch = [];
        for (let j = 0; j < CONCURRENT_REQUESTS && i + j < NUM_REQUESTS; j++) {
            batch.push(makeRequest());
        }
        const batchResults = await Promise.all(batch);
        results.push(...batchResults);
    }
    
    const successCount = results.filter(r => r.success).length;
    const avgLatency = results.reduce((sum, r) => sum + r.latency, 0) / results.length;
    const maxLatency = Math.max(...results.map(r => r.latency));
    const minLatency = Math.min(...results.map(r => r.latency));
    
    console.log('Load Test Results:');
    console.log(`- Total requests: ${results.length}`);
    console.log(`- Successful requests: ${successCount} (${(successCount/results.length*100).toFixed(2)}%)`);
    console.log(`- Average latency: ${avgLatency.toFixed(2)}ms`);
    console.log(`- Min latency: ${minLatency}ms`);
    console.log(`- Max latency: ${maxLatency}ms`);
}

runLoadTest().catch(console.error);
EOF

    # Start API server in background for load testing
    cd api-server
    npm run dev &
    API_SERVER_PID=$!
    cd ..
    
    sleep 5 # Give server time to start
    
    node load_test.js
    
    # Clean up
    kill $API_SERVER_PID 2>/dev/null || true
    rm -f load_test.js
    
    print_success "Load tests completed"
}

# Main execution
main() {
    echo "Starting at $(date)"
    
    check_dependencies
    build_rust_components
    install_node_dependencies
    start_test_services
    
    # Run all tests
    run_rust_tests
    run_node_tests
    run_benchmarks
    run_load_tests
    
    stop_test_services
    generate_report
    
    print_success "All tests completed successfully! 🎉"
    echo "Check test-reports/summary.md for detailed results."
}

# Parse command line arguments
case "${1:-all}" in
    "rust")
        check_dependencies
        build_rust_components
        run_rust_tests
        ;;
    "node")
        check_dependencies
        install_node_dependencies
        run_node_tests
        ;;
    "load")
        run_load_tests
        ;;
    "bench")
        run_benchmarks
        ;;
    "all")
        main
        ;;
    *)
        echo "Usage: $0 [rust|node|load|bench|all]"
        echo "  rust  - Run only Rust tests"
        echo "  node  - Run only Node.js tests"
        echo "  load  - Run only load tests"
        echo "  bench - Run only benchmarks"
        echo "  all   - Run all tests (default)"
        exit 1
        ;;
esac
