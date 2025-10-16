# Metrics API Documentation

The API server exposes comprehensive metrics endpoints for the Next.js dashboard.

## Base URL
```
http://localhost:3001
```

## Available Endpoints

### 1. Comprehensive Metrics
```
GET /api/metrics
```
Returns all metrics in a single response:
- Bundle statistics (total, fees, averages, recent bundles)
- Auction statistics (total auctions, winners, highest tips)
- System metrics (uptime, current window, Redis status)

### 2. Bundle-Specific Metrics
```
GET /api/metrics/bundles
```
Returns detailed bundle statistics:
- `totalBundles`: Total number of bundles received
- `totalFees`: Sum of all bundle tips
- `averageFee`: Average tip per bundle
- `bundlesPerMinute`: Recent bundle submission rate
- `recentBundles`: Array of recent bundle submissions with details

### 3. Auction-Specific Metrics
```
GET /api/metrics/auctions
```
Returns auction and winner statistics:
- `totalAuctions`: Total number of auctions conducted
- `totalWinners`: Total number of auction winners
- `averageWinnersPerAuction`: Average winners per auction
- `highestTip`: Highest tip recorded
- `recentAuctions`: Array of recent auction results

### 4. System Metrics
```
GET /api/metrics/system
```
Returns system health and operational metrics:
- `uptime`: Server uptime in milliseconds
- `currentTimestamp`: Current server timestamp
- `currentWindowId`: Current auction window ID
- `redisConnected`: Redis connection status

### 5. Reset Metrics (Development/Testing)
```
POST /api/metrics/reset
```
Resets all metrics to zero (useful for testing and development).

## Example Response Format

All endpoints return data in this format:
```json
{
  "success": true,
  "data": {
    // Endpoint-specific data
  },
  "timestamp": "2025-10-16T03:51:15.827Z"
}
```

## Bundle Submission

To test metrics, submit bundles via:
```
POST /api/bundles
Content-Type: application/json

{
  "tip": 1500,
  "searcher_pubkey": "searcher_001",
  "transactions": ["tx1", "tx2", "tx3"]
}
```

## Real-time Updates

The metrics are updated in real-time as bundles are submitted and auctions are conducted. The Next.js dashboard can poll these endpoints or implement WebSocket connections for live updates.

## Dependencies

- Redis server must be running for metrics persistence
- Metrics are stored in Redis with configurable history limits
- All monetary values are in the base currency units (e.g., lamports for Solana)
