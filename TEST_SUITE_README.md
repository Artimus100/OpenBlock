# Permissionless Block Builder - Testing Suite

## Overview
This comprehensive testing suite provides extensive coverage for the Solana permissionless block builder system, including unit tests, integration tests, benchmarks, and helper scripts for simulation and deployment.

## Test Components

### 1. Rust Tests (Block Engine)

#### Core Module Tests
- **Bundle Module**: Tests bundle creation, validation, and error handling
- **Transaction Pool**: Tests concurrent access, event notifications, and tip-based queries  
- **Auction Engine**: Tests bid ordering, simulation filtering, and winner selection
- **Block Assembler**: Tests transaction limits, block validation, and assembly
- **Simulator**: Tests transaction simulation with mock Solana RPC

#### Integration Tests
- End-to-end pipeline testing from bundle submission to block assembly
- Concurrency tests with multiple simultaneous bundle submissions
- Performance benchmarks measuring latency at each stage
- Error handling and recovery scenarios

#### Property-Based Tests (Optional)
- Uses `proptest` for generating randomized test cases
- Validates invariants across different input combinations
- Ensures robustness under edge conditions

#### Benchmarks
- Criterion-based performance benchmarks
- Measures throughput and latency for each component
- Scales testing from small to large workloads

### 2. Node.js API Tests

#### API Endpoint Tests
- Bundle submission endpoint testing
- Status query endpoint validation
- Error response handling
- Load testing with concurrent requests

#### Integration with Mock Services
- Redis connection mocking
- Database interaction testing
- External service simulation

### 3. Helper Scripts

#### Test Runner (`scripts/run_tests.sh`)
Comprehensive test execution script with multiple modes:

```bash
# Run all tests
./scripts/run_tests.sh all

# Run only Rust tests
./scripts/run_tests.sh rust

# Run only Node.js tests  
./scripts/run_tests.sh node

# Run load tests
./scripts/run_tests.sh load

# Run benchmarks
./scripts/run_tests.sh bench
```

**Features:**
- Automatic dependency checking
- Code coverage generation
- Performance benchmarking
- Test report generation
- Service lifecycle management

#### Bundle Simulator (`scripts/simulate_bundles.sh`)
Simulates realistic bundle submission scenarios:

```bash
# Run normal simulation
./scripts/simulate_bundles.sh simulate

# Run benchmark mode
./scripts/simulate_bundles.sh benchmark

# Run stress test
./scripts/simulate_bundles.sh stress

# Environment variables
export NUM_SEARCHERS=10           # Number of concurrent searchers
export BUNDLES_PER_SEARCHER=20    # Bundles per searcher
export MIN_TIP=1000000            # Minimum tip in lamports
export MAX_TIP=10000000           # Maximum tip in lamports
```

**Capabilities:**
- Multi-searcher simulation
- Varying tip amounts and transaction counts
- Concurrent bundle submission
- Real-time status monitoring
- Performance metrics collection

#### Testnet Integration (`scripts/testnet_integration.sh`)
Sets up local Solana testnet for realistic testing:

```bash
# Full integration test suite
./scripts/testnet_integration.sh full

# Setup only
./scripts/testnet_integration.sh setup

# Run tests against existing testnet
./scripts/testnet_integration.sh test

# Monitoring mode
./scripts/testnet_integration.sh monitor
```

**Features:**
- Automatic Solana CLI installation
- Local test validator setup
- Test account creation and funding
- Real transaction simulation
- System monitoring and reporting

## Running the Test Suite

### Prerequisites
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Node.js and npm
# (Platform specific - see nodejs.org)

# Install additional tools (optional)
cargo install cargo-tarpaulin  # For Rust code coverage
npm install -g @solana/web3.js # For Solana integration
```

### Quick Start
```bash
# Navigate to project root
cd /path/to/permissionless-block-builder

# Make scripts executable
chmod +x scripts/*.sh

# Run comprehensive test suite
./scripts/run_tests.sh all

# View test results
cat test-reports/summary.md
```

### Individual Test Categories

#### Unit Tests
```bash
# Rust unit tests
cd block-engine
cargo test --lib

# Node.js unit tests
cd api-server
npm test
```

#### Integration Tests
```bash
# Rust integration tests
cd block-engine
cargo test integration_tests

# Full pipeline integration
./scripts/testnet_integration.sh full
```

#### Performance Tests
```bash
# Rust benchmarks
cd block-engine
cargo bench

# Load testing
./scripts/simulate_bundles.sh benchmark
```

#### End-to-End Tests
```bash
# With local testnet
./scripts/testnet_integration.sh full

# With mock services
./scripts/run_tests.sh all
```

## Test Coverage

### Rust Components
- **Bundle validation**: Empty bundles, transaction limits, tip validation
- **Pool management**: Concurrency, capacity limits, event notifications
- **Auction mechanics**: Tip-based ordering, simulation filtering
- **Block assembly**: Transaction limits, compute unit limits, validation
- **Simulation**: Success/failure scenarios, deterministic results

### API Endpoints
- **POST /api/bundles**: Bundle submission, validation, error handling
- **GET /api/bundles/:id**: Status queries, not found scenarios
- **GET /api/status/metrics**: System metrics, performance monitoring
- **GET /health**: Health check endpoint

### Integration Scenarios
- Bundle submission → Pool → Auction → Block Assembly → Validator Submission
- Concurrent bundle processing under load
- Error propagation and recovery
- Performance under varying load conditions

## Performance Benchmarks

### Latency Targets
- Bundle submission: < 10ms
- Auction processing: < 20ms  
- Block assembly: < 50ms
- End-to-end pipeline: < 100ms

### Throughput Targets
- Bundle processing: > 1000 bundles/second
- API requests: > 500 requests/second
- Concurrent connections: > 100 simultaneous

### Load Testing Results
The test suite generates detailed performance reports including:
- Request/response latency distributions
- Throughput measurements under load
- Resource utilization metrics
- Error rates and failure scenarios

## Error Scenarios Tested

### Bundle Validation Errors
- Empty bundle submission
- Excessive transaction count
- Invalid transaction signatures
- Simulation failures

### System Errors
- Pool capacity exceeded
- Network connectivity issues
- Database connection failures
- Invalid configuration

### Recovery Testing
- Service restart scenarios
- Network partition recovery
- Database failover
- Load balancer failures

## Configuration

### Environment Variables
```bash
# API Configuration
API_BASE_URL=http://localhost:3001
API_SERVER_PORT=3001

# Test Configuration  
NUM_SEARCHERS=5
BUNDLES_PER_SEARCHER=10
MIN_TIP=1000000
MAX_TIP=10000000
SIMULATION_DURATION=60

# Solana Configuration
SOLANA_VERSION=1.18.0
TESTNET_RPC_PORT=8899
BLOCK_ENGINE_PORT=8080
```

### Test Data
- Mock transaction generation
- Randomized tip amounts
- Varied bundle sizes
- Simulated searcher identities

## Monitoring and Observability

### Metrics Collection
- Bundle processing rates
- Auction selection statistics  
- Block assembly performance
- API response times

### Logging
- Structured logging with tracing
- Error tracking and aggregation
- Performance metrics
- Debug information

### Reports
- Test execution summaries
- Coverage reports (HTML/LCOV)
- Performance benchmarks
- System health checks

## Continuous Integration

### Test Pipeline
1. Code compilation and basic checks
2. Unit test execution
3. Integration test suite
4. Performance benchmark comparison
5. Security and dependency scanning
6. Test report generation

### Quality Gates
- All tests must pass
- Code coverage > 80%
- Performance within acceptable ranges
- No critical security vulnerabilities

## Troubleshooting

### Common Issues
- **Port conflicts**: Ensure test ports are available
- **Permission errors**: Run scripts with appropriate permissions
- **Memory issues**: Adjust test parameters for available resources
- **Network timeouts**: Check firewall and network configuration

### Debug Mode
```bash
# Enable verbose logging
RUST_LOG=debug ./scripts/run_tests.sh rust

# Run individual test with output
cargo test test_name -- --nocapture

# API server debug mode
DEBUG=* npm test
```

## Contributing

### Adding New Tests
1. Follow existing test patterns and naming conventions
2. Include both positive and negative test cases
3. Add performance benchmarks for new features
4. Update documentation and test reports

### Test Data Management
- Use deterministic random data where possible
- Clean up test artifacts after execution
- Avoid hardcoded values in test assertions
- Document test data requirements

This comprehensive testing suite ensures the reliability, performance, and correctness of the permissionless block builder under various conditions and load scenarios.
