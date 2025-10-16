import Redis from 'ioredis';

const redis = new Redis(process.env.REDIS_URL || "redis://localhost:6379");

export interface BundleMetrics {
  totalBundles: number;
  totalFees: number;
  averageFee: number;
  bundlesPerMinute: number;
  recentBundles: Array<{
    id: string;
    tip: number;
    searcher_pubkey: string;
    timestamp: number;
  }>;
}

export interface AuctionMetrics {
  totalAuctions: number;
  totalWinners: number;
  averageWinnersPerAuction: number;
  highestTip: number;
  recentAuctions: Array<{
    window_id: number;
    winners: number;
    totalTip: number;
    averageTip: number;
    timestamp: number;
  }>;
}

export interface SystemMetrics {
  uptime: number;
  currentTimestamp: number;
  currentWindowId: number;
  redisConnected: boolean;
}

class MetricsService {
  private readonly METRICS_KEY = 'block_builder_metrics';
  private readonly BUNDLE_HISTORY_KEY = 'bundle_history';
  private readonly AUCTION_HISTORY_KEY = 'auction_history';
  private readonly MAX_HISTORY_ITEMS = 100;
  private startTime: number;

  constructor() {
    this.startTime = Date.now();
  }

  async recordBundle(bundle: any): Promise<void> {
    try {
      // Store bundle in history
      const bundleRecord = {
        id: bundle.id,
        tip: bundle.tip,
        searcher_pubkey: bundle.searcher_pubkey,
        timestamp: bundle.timestamp || Date.now(),
      };

      await redis.lpush(this.BUNDLE_HISTORY_KEY, JSON.stringify(bundleRecord));
      await redis.ltrim(this.BUNDLE_HISTORY_KEY, 0, this.MAX_HISTORY_ITEMS - 1);

      // Update running totals
      await redis.hincrby(this.METRICS_KEY, 'total_bundles', 1);
      await redis.hincrby(this.METRICS_KEY, 'total_fees', bundle.tip);

      console.log(`📊 Recorded bundle ${bundle.id} with tip ${bundle.tip}`);
    } catch (error) {
      console.error('Failed to record bundle metrics:', error);
    }
  }

  async recordAuction(windowId: number, winners: any[]): Promise<void> {
    try {
      const totalTip = winners.reduce((sum, winner) => sum + winner.tip, 0);
      const averageTip = winners.length > 0 ? totalTip / winners.length : 0;

      const auctionRecord = {
        window_id: windowId,
        winners: winners.length,
        totalTip,
        averageTip,
        timestamp: Date.now(),
      };

      await redis.lpush(this.AUCTION_HISTORY_KEY, JSON.stringify(auctionRecord));
      await redis.ltrim(this.AUCTION_HISTORY_KEY, 0, this.MAX_HISTORY_ITEMS - 1);

      // Update running totals
      await redis.hincrby(this.METRICS_KEY, 'total_auctions', 1);
      await redis.hincrby(this.METRICS_KEY, 'total_winners', winners.length);

      // Track highest tip
      const currentHighest = await redis.hget(this.METRICS_KEY, 'highest_tip');
      const maxTip = Math.max(...winners.map(w => w.tip), 0);
      if (!currentHighest || maxTip > parseInt(currentHighest)) {
        await redis.hset(this.METRICS_KEY, 'highest_tip', maxTip);
      }

      console.log(`🏆 Recorded auction ${windowId} with ${winners.length} winners, total tip: ${totalTip}`);
    } catch (error) {
      console.error('Failed to record auction metrics:', error);
    }
  }

  async getBundleMetrics(): Promise<BundleMetrics> {
    try {
      const [totalBundles, totalFees, recentBundlesData] = await Promise.all([
        redis.hget(this.METRICS_KEY, 'total_bundles'),
        redis.hget(this.METRICS_KEY, 'total_fees'),
        redis.lrange(this.BUNDLE_HISTORY_KEY, 0, 9), // Last 10 bundles
      ]);

      const bundleCount = parseInt(totalBundles || '0');
      const feeSum = parseInt(totalFees || '0');
      const averageFee = bundleCount > 0 ? feeSum / bundleCount : 0;

      // Calculate bundles per minute from recent history
      const recentBundles = recentBundlesData.map(data => JSON.parse(data));
      const now = Date.now();
      const oneMinuteAgo = now - 60000;
      const recentCount = recentBundles.filter(b => b.timestamp > oneMinuteAgo).length;

      return {
        totalBundles: bundleCount,
        totalFees: feeSum,
        averageFee: Math.round(averageFee),
        bundlesPerMinute: recentCount,
        recentBundles: recentBundles.slice(0, 5), // Last 5 bundles
      };
    } catch (error) {
      console.error('Failed to get bundle metrics:', error);
      return {
        totalBundles: 0,
        totalFees: 0,
        averageFee: 0,
        bundlesPerMinute: 0,
        recentBundles: [],
      };
    }
  }

  async getAuctionMetrics(): Promise<AuctionMetrics> {
    try {
      const [totalAuctions, totalWinners, highestTip, recentAuctionsData] = await Promise.all([
        redis.hget(this.METRICS_KEY, 'total_auctions'),
        redis.hget(this.METRICS_KEY, 'total_winners'),
        redis.hget(this.METRICS_KEY, 'highest_tip'),
        redis.lrange(this.AUCTION_HISTORY_KEY, 0, 9), // Last 10 auctions
      ]);

      const auctionCount = parseInt(totalAuctions || '0');
      const winnerCount = parseInt(totalWinners || '0');
      const averageWinnersPerAuction = auctionCount > 0 ? winnerCount / auctionCount : 0;

      const recentAuctions = recentAuctionsData.map(data => JSON.parse(data));

      return {
        totalAuctions: auctionCount,
        totalWinners: winnerCount,
        averageWinnersPerAuction: Math.round(averageWinnersPerAuction * 100) / 100,
        highestTip: parseInt(highestTip || '0'),
        recentAuctions: recentAuctions.slice(0, 5), // Last 5 auctions
      };
    } catch (error) {
      console.error('Failed to get auction metrics:', error);
      return {
        totalAuctions: 0,
        totalWinners: 0,
        averageWinnersPerAuction: 0,
        highestTip: 0,
        recentAuctions: [],
      };
    }
  }

  async getSystemMetrics(): Promise<SystemMetrics> {
    try {
      const currentTimestamp = Date.now();
      const uptime = currentTimestamp - this.startTime;
      const currentWindowId = Math.floor(currentTimestamp / 200);
      
      // Test Redis connection
      let redisConnected = false;
      try {
        await redis.ping();
        redisConnected = true;
      } catch {
        redisConnected = false;
      }

      return {
        uptime,
        currentTimestamp,
        currentWindowId,
        redisConnected,
      };
    } catch (error) {
      console.error('Failed to get system metrics:', error);
      return {
        uptime: 0,
        currentTimestamp: Date.now(),
        currentWindowId: 0,
        redisConnected: false,
      };
    }
  }

  async getAllMetrics() {
    const [bundleMetrics, auctionMetrics, systemMetrics] = await Promise.all([
      this.getBundleMetrics(),
      this.getAuctionMetrics(),
      this.getSystemMetrics(),
    ]);

    return {
      bundles: bundleMetrics,
      auctions: auctionMetrics,
      system: systemMetrics,
      timestamp: Date.now(),
    };
  }

  async resetMetrics(): Promise<void> {
    try {
      await Promise.all([
        redis.del(this.METRICS_KEY),
        redis.del(this.BUNDLE_HISTORY_KEY),
        redis.del(this.AUCTION_HISTORY_KEY),
      ]);
      console.log('🔄 Metrics reset successfully');
    } catch (error) {
      console.error('Failed to reset metrics:', error);
    }
  }
}

export const metricsService = new MetricsService();
