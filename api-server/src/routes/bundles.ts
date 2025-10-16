import express from "express";
import Redis from "ioredis";
import crypto from "crypto";
import { metricsService } from "../services/metrics";

const router = express.Router();
const redis = new Redis(process.env.REDIS_URL || "redis://localhost:6379");

// --- POST /api/bundles ---
router.post("/", async (req, res) => {
  try {
    const { transactions, tip, searcher_pubkey } = req.body;

    if (!transactions || !Array.isArray(transactions) || !tip || !searcher_pubkey) {
      return res.status(400).json({ error: "Invalid bundle format" });
    }

    // Generate bundle ID
    const bundle_id = crypto.randomBytes(8).toString("hex");

    const bundle = {
      id: bundle_id,
      transactions,
      tip,
      searcher_pubkey,
      timestamp: Date.now(),
    };

    // Push to Redis queue for the current time window
    const window_id = Math.floor(Date.now() / 200); // 200ms "slot window"
    await redis.rpush(`bundle_window:${window_id}`, JSON.stringify(bundle));

    // Record bundle metrics
    await metricsService.recordBundle(bundle);

    return res.status(200).json({ status: "queued", bundle_id, window_id });
  } catch (err) {
    console.error("Bundle submit error:", err);
    return res.status(500).json({ error: "Internal Server Error" });
  }
});

export default router;
