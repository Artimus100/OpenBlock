import { treasuryService } from '../services/treasury';

// Simulate auction results for treasury tracking
async function simulateAuctions() {
  console.log('🎯 Starting auction simulation for treasury tracking...\n');

  // Simulate 5 auctions with different fee patterns
  const auctionScenarios = [
    // Auction 1: Low activity
    {
      auctionId: 1001,
      bundles: [
        { tip: 1500, searcher: 'searcher_001' },
        { tip: 1200, searcher: 'searcher_002' }
      ]
    },
    // Auction 2: Medium activity
    {
      auctionId: 1002,
      bundles: [
        { tip: 2500, searcher: 'searcher_003' },
        { tip: 1800, searcher: 'searcher_004' },
        { tip: 3200, searcher: 'searcher_005' },
        { tip: 1600, searcher: 'searcher_006' }
      ]
    },
    // Auction 3: High activity
    {
      auctionId: 1003,
      bundles: [
        { tip: 5000, searcher: 'searcher_007' },
        { tip: 4500, searcher: 'searcher_008' },
        { tip: 3800, searcher: 'searcher_009' },
        { tip: 2200, searcher: 'searcher_010' },
        { tip: 1900, searcher: 'searcher_011' },
        { tip: 2700, searcher: 'searcher_012' }
      ]
    },
    // Auction 4: Premium activity
    {
      auctionId: 1004,
      bundles: [
        { tip: 8000, searcher: 'searcher_013' },
        { tip: 7500, searcher: 'searcher_014' },
        { tip: 6200, searcher: 'searcher_015' }
      ]
    },
    // Auction 5: Mixed activity
    {
      auctionId: 1005,
      bundles: [
        { tip: 1000, searcher: 'searcher_016' },
        { tip: 4500, searcher: 'searcher_017' },
        { tip: 2300, searcher: 'searcher_018' },
        { tip: 1700, searcher: 'searcher_019' },
        { tip: 3100, searcher: 'searcher_020' }
      ]
    }
  ];

  // Process each auction
  for (const scenario of auctionScenarios) {
    console.log(`🏆 Processing Auction ${scenario.auctionId}`);
    console.log(`   Bundles: ${scenario.bundles.length}`);
    console.log(`   Total Fees: ${scenario.bundles.reduce((sum, b) => sum + b.tip, 0)}`);
    
    await treasuryService.recordAuctionFees(scenario.auctionId, scenario.bundles);
    
    // Small delay between auctions
    await new Promise(resolve => setTimeout(resolve, 500));
  }

  console.log('\n📊 Treasury Simulation Complete!\n');

  // Display final treasury stats
  const stats = await treasuryService.getTreasuryStats();
  console.log('💰 Final Treasury Statistics:');
  console.log(`   Total Collected: ${stats.totalCollected} fees`);
  console.log(`   Total Auctions: ${stats.totalAuctions}`);
  console.log(`   Average per Auction: ${stats.averageFeesPerAuction.toFixed(2)} fees`);
  console.log(`   Growth Rate (last hour): ${stats.growthRate} fees`);
  
  console.log('\n📈 Recent Auction History:');
  stats.recentEntries.slice(0, 3).forEach((entry, i) => {
    console.log(`   ${i + 1}. Auction ${entry.auctionId}: ${entry.totalFees} fees (${entry.bundleCount} bundles)`);
  });

  console.log('\n🔗 Test treasury endpoints:');
  console.log('   GET  http://localhost:3001/api/metrics/treasury');
  console.log('   GET  http://localhost:3001/api/metrics/treasury/export?format=csv');
  console.log('   POST http://localhost:3001/api/metrics/treasury/reset');
}

// Run simulation if called directly
if (require.main === module) {
  simulateAuctions().catch(console.error);
}

export { simulateAuctions };
