import fs from 'fs/promises';
import path from 'path';

export interface TreasuryEntry {
  timestamp: number;
  auctionId: number;
  totalFees: number;
  bundleCount: number;
  averageFee: number;
  cumulativeFees: number;
}

export interface TreasuryData {
  totalCollected: number;
  totalAuctions: number;
  entries: TreasuryEntry[];
  lastUpdated: number;
}

class TreasuryService {
  private readonly treasuryFile: string;
  private readonly maxEntries = 1000; // Keep last 1000 auction entries
  
  constructor() {
    this.treasuryFile = path.join(process.cwd(), 'treasury.json');
  }

  async initializeTreasury(): Promise<void> {
    try {
      await fs.access(this.treasuryFile);
    } catch (error) {
      // File doesn't exist, create initial treasury
      const initialData: TreasuryData = {
        totalCollected: 0,
        totalAuctions: 0,
        entries: [],
        lastUpdated: Date.now()
      };
      await this.saveTreasuryData(initialData);
      console.log('📊 Treasury initialized');
    }
  }

  async recordAuctionFees(auctionId: number, bundles: any[]): Promise<void> {
    try {
      const treasuryData = await this.loadTreasuryData();
      
      const totalFees = bundles.reduce((sum, bundle) => sum + (bundle.tip || 0), 0);
      const averageFee = bundles.length > 0 ? totalFees / bundles.length : 0;
      
      const newEntry: TreasuryEntry = {
        timestamp: Date.now(),
        auctionId,
        totalFees,
        bundleCount: bundles.length,
        averageFee,
        cumulativeFees: treasuryData.totalCollected + totalFees
      };

      // Update treasury data
      treasuryData.totalCollected += totalFees;
      treasuryData.totalAuctions += 1;
      treasuryData.entries.unshift(newEntry); // Add to beginning
      treasuryData.lastUpdated = Date.now();

      // Trim entries to max limit
      if (treasuryData.entries.length > this.maxEntries) {
        treasuryData.entries = treasuryData.entries.slice(0, this.maxEntries);
      }

      await this.saveTreasuryData(treasuryData);
      
      console.log(`💰 Treasury updated: +${totalFees} fees (Total: ${treasuryData.totalCollected})`);
    } catch (error) {
      console.error('Failed to record auction fees:', error);
    }
  }

  async getTreasuryData(): Promise<TreasuryData> {
    return await this.loadTreasuryData();
  }

  async getTreasuryStats(): Promise<{
    totalCollected: number;
    totalAuctions: number;
    averageFeesPerAuction: number;
    recentEntries: TreasuryEntry[];
    growthRate: number; // Fees per hour
  }> {
    const data = await this.loadTreasuryData();
    
    // Calculate growth rate (fees per hour)
    const now = Date.now();
    const oneHourAgo = now - (60 * 60 * 1000);
    const recentEntries = data.entries.filter(entry => entry.timestamp > oneHourAgo);
    const recentFees = recentEntries.reduce((sum, entry) => sum + entry.totalFees, 0);
    
    return {
      totalCollected: data.totalCollected,
      totalAuctions: data.totalAuctions,
      averageFeesPerAuction: data.totalAuctions > 0 ? data.totalCollected / data.totalAuctions : 0,
      recentEntries: data.entries.slice(0, 10), // Last 10 auctions
      growthRate: recentFees // Fees collected in last hour
    };
  }

  async exportTreasuryHistory(format: 'json' | 'csv' = 'json'): Promise<string> {
    const data = await this.loadTreasuryData();
    
    if (format === 'csv') {
      const headers = ['timestamp', 'auctionId', 'totalFees', 'bundleCount', 'averageFee', 'cumulativeFees'];
      const csvRows = [
        headers.join(','),
        ...data.entries.map(entry => 
          `${entry.timestamp},${entry.auctionId},${entry.totalFees},${entry.bundleCount},${entry.averageFee},${entry.cumulativeFees}`
        )
      ];
      return csvRows.join('\n');
    }
    
    return JSON.stringify(data, null, 2);
  }

  async resetTreasury(): Promise<void> {
    const initialData: TreasuryData = {
      totalCollected: 0,
      totalAuctions: 0,
      entries: [],
      lastUpdated: Date.now()
    };
    await this.saveTreasuryData(initialData);
    console.log('🔄 Treasury reset');
  }

  private async loadTreasuryData(): Promise<TreasuryData> {
    try {
      const data = await fs.readFile(this.treasuryFile, 'utf-8');
      return JSON.parse(data);
    } catch (error) {
      // Return default data if file doesn't exist
      return {
        totalCollected: 0,
        totalAuctions: 0,
        entries: [],
        lastUpdated: Date.now()
      };
    }
  }

  private async saveTreasuryData(data: TreasuryData): Promise<void> {
    await fs.writeFile(this.treasuryFile, JSON.stringify(data, null, 2));
  }
}

export const treasuryService = new TreasuryService();
