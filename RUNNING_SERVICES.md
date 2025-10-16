# 🚀 Running All Services in Parallel

This guide shows you multiple ways to run all the Block Builder services simultaneously.

## 📋 Quick Start Options

### Option 1: Automated Script (Recommended)
```bash
# Start all services with monitoring
./run-all-services.sh start

# Or using npm
npm run services:start
```

### Option 2: Docker Compose
```bash
# Start all services in containers
docker-compose up -d

# View logs
docker-compose logs -f

# Stop services
docker-compose down
```

### Option 3: Manual Service Management
```bash
# Terminal 1: Redis
redis-server

# Terminal 2: API Server
cd api-server && npm run dev

# Terminal 3: Block Engine
cd block-engine && cargo run --bin auction_demo

# Terminal 4: Frontend (if available)
cd frontend-dashboard && npm run dev
```

## 🔧 Service Overview

| Service | Port | Description | Health Check |
|---------|------|-------------|--------------|
| **Redis** | 6379 | Data store for metrics/bundles | `redis-cli ping` |
| **API Server** | 3001 | TypeScript REST API | `curl localhost:3001/health` |
| **Mock Validator** | 4000 | Block validation simulation | `curl localhost:4000/submit_block` |
| **Block Engine** | 8080 | Rust auction/assembly logic | Console output |
| **Dashboard** | 3000 | Next.js frontend (optional) | `curl localhost:3000` |

## 📊 Available Endpoints

### Bundle Management
```bash
# Submit a bundle
curl -X POST http://localhost:3001/api/bundles \
  -H "Content-Type: application/json" \
  -d '{"tip": 1500, "searcher_pubkey": "test_searcher", "transactions": ["tx1", "tx2"]}'
```

### Metrics & Analytics
```bash
# Get all metrics
curl http://localhost:3001/api/metrics | jq .

# Get bundle metrics
curl http://localhost:3001/api/metrics/bundles | jq .

# Get auction metrics  
curl http://localhost:3001/api/metrics/auctions | jq .

# Get system metrics
curl http://localhost:3001/api/metrics/system | jq .
```

### Treasury Tracking
```bash
# Get treasury statistics
curl http://localhost:3001/api/metrics/treasury | jq .

# Export treasury history as CSV
curl 'http://localhost:3001/api/metrics/treasury/export?format=csv'

# Export as JSON
curl 'http://localhost:3001/api/metrics/treasury/export?format=json'
```

## 🎯 Service Management Commands

```bash
# Check service status
./run-all-services.sh status
# or
npm run services:status

# Stop all services
./run-all-services.sh clean
# or  
npm run services:stop

# Run treasury demo only
./run-all-services.sh demo
# or
npm run demo:treasury

# Docker commands
npm run docker:up      # Start with Docker
npm run docker:down    # Stop Docker services
npm run docker:logs    # View Docker logs
```

## 🔍 Monitoring & Debugging

### Service Health Checks
```bash
# Check if all services are running
lsof -i :3001,4000,6379,8080,3000

# Check specific service
curl -f http://localhost:3001/health
```

### Log Monitoring
```bash
# Follow API server logs (if using script)
tail -f /tmp/api-server.log

# Follow all Docker logs
docker-compose logs -f

# Check Redis logs
redis-cli monitor
```

### Treasury Data
```bash
# View treasury file directly
cat api-server/treasury.json | jq .

# Reset treasury for testing
curl -X POST http://localhost:3001/api/metrics/treasury/reset
```

## 🏗️ Development Workflow

### 1. Start All Services
```bash
./run-all-services.sh start
```

### 2. Submit Test Bundles
```bash
# High-value bundle
curl -X POST http://localhost:3001/api/bundles \
  -H "Content-Type: application/json" \
  -d '{"tip": 5000, "searcher_pubkey": "whale_searcher", "transactions": ["tx1", "tx2", "tx3"]}'

# Medium-value bundle  
curl -X POST http://localhost:3001/api/bundles \
  -H "Content-Type: application/json" \
  -d '{"tip": 2500, "searcher_pubkey": "med_searcher", "transactions": ["tx4", "tx5"]}'
```

### 3. Run Treasury Demo
```bash
npm run demo:treasury
```

### 4. Check Results
```bash
# View treasury growth
curl http://localhost:3001/api/metrics/treasury | jq '.data.totalCollected'

# View metrics
curl http://localhost:3001/api/metrics | jq '.data.bundles.totalBundles'
```

## 🐛 Troubleshooting

### Port Conflicts
```bash
# Kill processes on busy ports
./run-all-services.sh clean

# Check what's using a port
lsof -i :3001
```

### Redis Connection Issues
```bash
# Start Redis manually
redis-server --daemonize yes

# Test Redis connection
redis-cli ping
```

### Build Issues
```bash
# Rebuild Rust components
cargo clean && cargo build

# Reinstall Node dependencies
cd api-server && rm -rf node_modules && npm install
```

### Docker Issues
```bash
# Reset Docker environment
docker-compose down -v
docker-compose up --build -d
```

## 📈 Performance Testing

### Load Testing Bundles
```bash
# Submit 10 bundles rapidly
for i in {1..10}; do
  curl -X POST http://localhost:3001/api/bundles \
    -H "Content-Type: application/json" \
    -d "{\"tip\": $((RANDOM % 5000 + 1000)), \"searcher_pubkey\": \"load_test_$i\", \"transactions\": [\"tx$i\"]}" &
done
wait
```

### Monitor Performance
```bash
# Check response times
time curl http://localhost:3001/api/metrics

# Monitor system resources
top -pid $(pgrep -f "node.*api-server")
```

## 🔒 Production Considerations

1. **Environment Variables**: Set production Redis URL, database connections
2. **Security**: Enable HTTPS, rate limiting, authentication
3. **Monitoring**: Add Prometheus metrics, health checks
4. **Scaling**: Use Redis Cluster, load balancers
5. **Backup**: Regular treasury.json backups
6. **Logging**: Structured logging with timestamps

## 🆘 Need Help?

- Check service status: `./run-all-services.sh status`
- View logs: `docker-compose logs -f`  
- Reset everything: `./run-all-services.sh clean && ./run-all-services.sh start`
- Test endpoints: Use the curl commands above
