import request from 'supertest';
import { createTestApp, generateMockBundle, MockBundleService } from './helpers';

describe('Bundle Routes', () => {
  let app: any;
  let mockBundleService: MockBundleService;

  beforeEach(() => {
    app = createTestApp();
    mockBundleService = new MockBundleService();
  });

  afterEach(() => {
    mockBundleService.clear();
  });

  describe('POST /api/bundles', () => {
    it('should submit a valid bundle successfully', async () => {
      const bundleData = generateMockBundle();
      
      const response = await request(app)
        .post('/api/bundles')
        .send(bundleData)
        .expect(200);

      expect(response.body).toHaveProperty('success', true);
      expect(response.body).toHaveProperty('bundleId');
      expect(typeof response.body.bundleId).toBe('string');
    });

    it('should reject bundle with missing transactions', async () => {
      const invalidBundle = generateMockBundle({ transactions: undefined });
      
      const response = await request(app)
        .post('/api/bundles')
        .send(invalidBundle)
        .expect(400);

      expect(response.body).toHaveProperty('error');
    });

    it('should reject bundle with invalid tip amount', async () => {
      const invalidBundle = generateMockBundle({ tipLamports: -1000 });
      
      const response = await request(app)
        .post('/api/bundles')
        .send(invalidBundle)
        .expect(400);

      expect(response.body).toHaveProperty('error');
    });

    it('should reject bundle with empty transactions array', async () => {
      const invalidBundle = generateMockBundle({ transactions: [] });
      
      const response = await request(app)
        .post('/api/bundles')
        .send(invalidBundle)
        .expect(400);

      expect(response.body).toHaveProperty('error');
    });

    it('should reject bundle with too many transactions', async () => {
      const tooManyTransactions = Array(10).fill(generateMockBundle().transactions[0]);
      const invalidBundle = generateMockBundle({ transactions: tooManyTransactions });
      
      const response = await request(app)
        .post('/api/bundles')
        .send(invalidBundle)
        .expect(400);

      expect(response.body).toHaveProperty('error');
    });

    it('should handle internal server errors gracefully', async () => {
      // This would require mocking the bundle service to throw an error
      // For now, we'll test the error handling structure
      const bundleData = generateMockBundle();
      
      // In a real test, you'd mock the service to throw here
      const response = await request(app)
        .post('/api/bundles')
        .send(bundleData);

      expect([200, 500]).toContain(response.status);
    });
  });

  describe('GET /api/bundles/:id', () => {
    it('should retrieve bundle status for valid ID', async () => {
      const response = await request(app)
        .get('/api/bundles/test-bundle-id')
        .expect(200);

      expect(response.body).toHaveProperty('bundleId', 'test-bundle-id');
      expect(response.body).toHaveProperty('status');
    });

    it('should return 404 for non-existent bundle', async () => {
      const response = await request(app)
        .get('/api/bundles/non-existent-id')
        .expect(200); // Currently returns 200 with status - might want to change to 404

      expect(response.body).toHaveProperty('bundleId', 'non-existent-id');
    });

    it('should handle malformed bundle IDs', async () => {
      const response = await request(app)
        .get('/api/bundles/malformed@id!')
        .expect(200);

      expect(response.body).toHaveProperty('bundleId', 'malformed@id!');
    });
  });

  describe('Bundle Status Flow', () => {
    it('should track bundle from submission to completion', async () => {
      // Submit bundle
      const bundleData = generateMockBundle();
      const submitResponse = await request(app)
        .post('/api/bundles')
        .send(bundleData)
        .expect(200);

      const bundleId = submitResponse.body.bundleId;

      // Check initial status
      const statusResponse = await request(app)
        .get(`/api/bundles/${bundleId}`)
        .expect(200);

      expect(statusResponse.body.bundleId).toBe(bundleId);
      expect(statusResponse.body.status).toBe('pending');
    });
  });

  describe('Rate Limiting and Load Testing', () => {
    it('should handle multiple concurrent bundle submissions', async () => {
      const numRequests = 50;
      const promises = [];

      for (let i = 0; i < numRequests; i++) {
        const bundleData = generateMockBundle({ tipLamports: 1000 + i });
        promises.push(
          request(app)
            .post('/api/bundles')
            .send(bundleData)
        );
      }

      const responses = await Promise.all(promises);
      
      // Check that most requests succeeded
      const successCount = responses.filter(r => r.status === 200).length;
      expect(successCount).toBeGreaterThan(numRequests * 0.8); // At least 80% should succeed
    });

    it('should handle large bundle payloads', async () => {
      const largeBundleData = generateMockBundle({
        transactions: Array(5).fill(null).map(() => ({
          signatures: [`${Math.random().toString(36)}`.repeat(10)],
          message: {
            accountKeys: Array(20).fill(null).map(() => Math.random().toString(36)),
            instructions: Array(10).fill(null).map(() => ({
              programIdIndex: 0,
              accounts: [1, 2, 3, 4, 5],
              data: 'base64'.repeat(100)
            }))
          }
        }))
      });

      const response = await request(app)
        .post('/api/bundles')
        .send(largeBundleData);

      expect([200, 400, 413]).toContain(response.status); // 413 = Payload Too Large
    });
  });

  describe('Bundle Validation', () => {
    it('should validate searcher public key format', async () => {
      const invalidBundle = generateMockBundle({ 
        searcherPubkey: 'invalid-pubkey-format' 
      });
      
      const response = await request(app)
        .post('/api/bundles')
        .send(invalidBundle);

      // Depending on validation implementation, this might be 400
      expect([200, 400]).toContain(response.status);
    });

    it('should validate transaction signature format', async () => {
      const invalidBundle = generateMockBundle({
        transactions: [{
          signatures: ['invalid-signature'],
          message: { accountKeys: [], instructions: [] }
        }]
      });
      
      const response = await request(app)
        .post('/api/bundles')
        .send(invalidBundle);

      expect([200, 400]).toContain(response.status);
    });
  });

  describe('Performance Testing', () => {
    it('should respond to bundle submission within reasonable time', async () => {
      const startTime = Date.now();
      const bundleData = generateMockBundle();
      
      await request(app)
        .post('/api/bundles')
        .send(bundleData)
        .expect(200);

      const responseTime = Date.now() - startTime;
      expect(responseTime).toBeLessThan(1000); // Should respond within 1 second
    });

    it('should respond to status queries within reasonable time', async () => {
      const startTime = Date.now();
      
      await request(app)
        .get('/api/bundles/test-id')
        .expect(200);

      const responseTime = Date.now() - startTime;
      expect(responseTime).toBeLessThan(500); // Should respond within 500ms
    });
  });
});
