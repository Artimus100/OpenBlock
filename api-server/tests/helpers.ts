import request from 'supertest';
import express from 'express';
import bundleRoutes from '../src/routes/bundles';
import statusRoutes from '../src/routes/status';

// Mock data generators
export const generateMockBundle = (overrides: any = {}) => ({
  transactions: [
    {
      signatures: ['5j8WfcGJiHGPtcwkB6cdRjwvv7sGFWt8Y2JgkYVrNXYK3wRN9cZf1bH4X2VdE7e1F9mL3mD'],
      message: {
        accountKeys: ['2VMfyD7zF3e4iJZbY1Qp9HnKGrLm3W8QsZ7MQsL5kJ3M'],
        instructions: [
          {
            programIdIndex: 0,
            accounts: [1, 2],
            data: 'base64encodeddata'
          }
        ]
      }
    }
  ],
  tipLamports: 1000000,
  searcherPubkey: '9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM',
  ...overrides
});

export const generateMockBundleResponse = (overrides: any = {}) => ({
  id: 'bundle_12345',
  status: 'pending',
  submittedAt: new Date().toISOString(),
  ...overrides
});

// Test app setup
export const createTestApp = () => {
  const app = express();
  app.use(express.json());
  app.use('/api/bundles', bundleRoutes);
  app.use('/api/status', statusRoutes);
  return app;
};

// Mock bundle service
export class MockBundleService {
  private bundles: Map<string, any> = new Map();
  private shouldFail = false;

  setFailureMode(shouldFail: boolean) {
    this.shouldFail = shouldFail;
  }

  async submitBundle(bundle: any): Promise<{ success: boolean; bundleId?: string; error?: string }> {
    if (this.shouldFail) {
      return { success: false, error: 'Mock failure' };
    }

    const bundleId = `bundle_${Date.now()}_${Math.random().toString(36).substring(7)}`;
    this.bundles.set(bundleId, {
      ...bundle,
      id: bundleId,
      status: 'pending',
      submittedAt: new Date().toISOString()
    });

    return { success: true, bundleId };
  }

  async getBundleStatus(bundleId: string): Promise<{ bundleId: string; status: string; submittedAt?: string }> {
    const bundle = this.bundles.get(bundleId);
    if (!bundle) {
      throw new Error('Bundle not found');
    }

    return {
      bundleId: bundle.id,
      status: bundle.status,
      submittedAt: bundle.submittedAt
    };
  }

  async updateBundleStatus(bundleId: string, status: string) {
    const bundle = this.bundles.get(bundleId);
    if (bundle) {
      bundle.status = status;
    }
  }

  getBundles(): any[] {
    return Array.from(this.bundles.values());
  }

  clear() {
    this.bundles.clear();
  }
}

// Mock RPC client
export class MockSolanaRpcClient {
  private shouldFailSimulation = false;
  private latency = 0;

  setSimulationFailure(shouldFail: boolean) {
    this.shouldFailSimulation = shouldFail;
  }

  setLatency(ms: number) {
    this.latency = ms;
  }

  async simulateTransaction(transaction: any) {
    if (this.latency > 0) {
      await new Promise(resolve => setTimeout(resolve, this.latency));
    }

    if (this.shouldFailSimulation) {
      return {
        success: false,
        error: 'Simulation failed',
        logs: ['Program execution failed']
      };
    }

    return {
      success: true,
      logs: ['Program log: Success'],
      unitsConsumed: 5000
    };
  }

  async getLatestBlockhash() {
    return {
      blockhash: '5YsJzQzGUJ2hK9k3wX2bZv7QsL5nJ3mD8f1gH4vB2cE9',
      lastValidBlockHeight: 123456789
    };
  }
}

// Test utilities
export const waitForAsync = (ms: number = 100) => new Promise(resolve => setTimeout(resolve, ms));

export const createRandomString = (length: number = 10) => 
  Math.random().toString(36).substring(2, 2 + length);
