
import express from 'express';
import cors from 'cors';
import helmet from 'helmet';
import morgan from 'morgan';
import bundleRoutes from './routes/bundles';
import statusRoutes from './routes/status';

const app = express();
const PORT = process.env.PORT || 3001;

// Middleware
app.use(helmet());
app.use(cors());
app.use(morgan('combined'));
app.use(express.json());

// Routes
app.use('/api/bundles', bundleRoutes);
app.use('/api/status', statusRoutes);

app.get('/health', (req, res) => {
  res.json({ status: 'healthy', timestamp: new Date().toISOString() });
});

app.listen(PORT, () => {
  console.log(`API server running on port ${PORT}`);
});


app.post("/submit_block", (req, res) => {
  const { window_id, ordered_hash } = req.body;
  console.log(`🧩 Validator received block: window=${window_id} hash=${ordered_hash.slice(0, 16)}`);
  res.json({ status: "accepted", slot: window_id });
});

app.listen(4000, () => console.log("🪄 Mock validator running on port 4000"));
