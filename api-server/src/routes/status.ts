import express from 'express';

const router = express.Router();

router.get('/metrics', async (req, res) => {
  try {
    res.json({
      totalBundles: 0,
      successRate: 0,
      avgLatency: 0,
      activeValidators: 0
    });
  } catch (error) {
    res.status(500).json({ error: 'Failed to get metrics' });
  }
});

export default router;
