import express from 'express';
import { metricsService } from '../services/metrics';
import { treasuryService } from '../services/treasury';

const router = express.Router();

// GET /api/metrics - Returns comprehensive metrics for the dashboard
router.get('/', async (req, res) => {
  try {
    const metrics = await metricsService.getAllMetrics();
    
    res.json({
      success: true,
      data: metrics,
      timestamp: new Date().toISOString(),
    });
  } catch (error) {
    console.error('Failed to get metrics:', error);
    res.status(500).json({
      success: false,
      error: 'Failed to retrieve metrics',
      timestamp: new Date().toISOString(),
    });
  }
});

// GET /api/metrics/bundles - Returns bundle-specific metrics
router.get('/bundles', async (req, res) => {
  try {
    const bundleMetrics = await metricsService.getBundleMetrics();
    
    res.json({
      success: true,
      data: bundleMetrics,
      timestamp: new Date().toISOString(),
    });
  } catch (error) {
    console.error('Failed to get bundle metrics:', error);
    res.status(500).json({
      success: false,
      error: 'Failed to retrieve bundle metrics',
      timestamp: new Date().toISOString(),
    });
  }
});

// GET /api/metrics/auctions - Returns auction-specific metrics
router.get('/auctions', async (req, res) => {
  try {
    const auctionMetrics = await metricsService.getAuctionMetrics();
    
    res.json({
      success: true,
      data: auctionMetrics,
      timestamp: new Date().toISOString(),
    });
  } catch (error) {
    console.error('Failed to get auction metrics:', error);
    res.status(500).json({
      success: false,
      error: 'Failed to retrieve auction metrics',
      timestamp: new Date().toISOString(),
    });
  }
});

// GET /api/metrics/system - Returns system-specific metrics
router.get('/system', async (req, res) => {
  try {
    const systemMetrics = await metricsService.getSystemMetrics();
    
    res.json({
      success: true,
      data: systemMetrics,
      timestamp: new Date().toISOString(),
    });
  } catch (error) {
    console.error('Failed to get system metrics:', error);
    res.status(500).json({
      success: false,
      error: 'Failed to retrieve system metrics',
      timestamp: new Date().toISOString(),
    });
  }
});

// POST /api/metrics/reset - Resets all metrics (useful for testing)
router.post('/reset', async (req, res) => {
  try {
    await metricsService.resetMetrics();
    
    res.json({
      success: true,
      message: 'Metrics reset successfully',
      timestamp: new Date().toISOString(),
    });
  } catch (error) {
    console.error('Failed to reset metrics:', error);
    res.status(500).json({
      success: false,
      error: 'Failed to reset metrics',
      timestamp: new Date().toISOString(),
    });
  }
});

// GET /api/metrics/treasury - Returns treasury statistics
router.get('/treasury', async (req, res) => {
  try {
    const treasuryStats = await treasuryService.getTreasuryStats();
    
    res.json({
      success: true,
      data: treasuryStats,
      timestamp: new Date().toISOString(),
    });
  } catch (error) {
    console.error('Failed to get treasury stats:', error);
    res.status(500).json({
      success: false,
      error: 'Failed to retrieve treasury statistics',
      timestamp: new Date().toISOString(),
    });
  }
});

// GET /api/metrics/treasury/export - Export treasury history
router.get('/treasury/export', async (req, res) => {
  try {
    const format = req.query.format as 'json' | 'csv' || 'json';
    const data = await treasuryService.exportTreasuryHistory(format);
    
    if (format === 'csv') {
      res.setHeader('Content-Type', 'text/csv');
      res.setHeader('Content-Disposition', 'attachment; filename="treasury-history.csv"');
    } else {
      res.setHeader('Content-Type', 'application/json');
    }
    
    res.send(data);
  } catch (error) {
    console.error('Failed to export treasury history:', error);
    res.status(500).json({
      success: false,
      error: 'Failed to export treasury history',
      timestamp: new Date().toISOString(),
    });
  }
});

// POST /api/metrics/treasury/reset - Reset treasury (testing only)
router.post('/treasury/reset', async (req, res) => {
  try {
    await treasuryService.resetTreasury();
    
    res.json({
      success: true,
      message: 'Treasury reset successfully',
      timestamp: new Date().toISOString(),
    });
  } catch (error) {
    console.error('Failed to reset treasury:', error);
    res.status(500).json({
      success: false,
      error: 'Failed to reset treasury',
      timestamp: new Date().toISOString(),
    });
  }
});

export default router;
