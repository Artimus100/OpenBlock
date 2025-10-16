import request from 'supertest';
import { createTestApp } from './helpers';

describe('Status Routes', () => {
  let app: any;

  beforeEach(() => {
    app = createTestApp();
  });

  describe('GET /api/status/metrics', () => {
    it('should return system metrics', async () => {
      const response = await request(app)
        .get('/api/status/metrics')
        .expect(200);

      expect(response.body).toHaveProperty('totalBundles');
      expect(response.body).toHaveProperty('successRate');
      expect(response.body).toHaveProperty('avgLatency');
      expect(response.body).toHaveProperty('activeValidators');

      // Validate metric types
      expect(typeof response.body.totalBundles).toBe('number');
      expect(typeof response.body.successRate).toBe('number');
      expect(typeof response.body.avgLatency).toBe('number');
      expect(typeof response.body.activeValidators).toBe('number');
    });

    it('should return metrics within expected ranges', async () => {
      const response = await request(app)
        .get('/api/status/metrics')
        .expect(200);

      const { totalBundles, successRate, avgLatency, activeValidators } = response.body;

      expect(totalBundles).toBeGreaterThanOrEqual(0);
      expect(successRate).toBeGreaterThanOrEqual(0);
      expect(successRate).toBeLessThanOrEqual(100);
      expect(avgLatency).toBeGreaterThanOrEqual(0);
      expect(activeValidators).toBeGreaterThanOrEqual(0);
    });

    it('should respond quickly to metrics requests', async () => {
      const startTime = Date.now();
      
      await request(app)
        .get('/api/status/metrics')
        .expect(200);

      const responseTime = Date.now() - startTime;
      expect(responseTime).toBeLessThan(200); // Should respond within 200ms
    });
  });

  describe('Health Check Integration', () => {
    it('should provide health status endpoint', async () => {
      const response = await request(app)
        .get('/health')
        .expect(200);

      expect(response.body).toHaveProperty('status', 'healthy');
      expect(response.body).toHaveProperty('timestamp');
    });
  });

  describe('Load Testing Status Endpoints', () => {
    it('should handle multiple concurrent metrics requests', async () => {
      const numRequests = 20;
      const promises = [];

      for (let i = 0; i < numRequests; i++) {
        promises.push(
          request(app).get('/api/status/metrics')
        );
      }

      const responses = await Promise.all(promises);
      
      // All requests should succeed
      responses.forEach(response => {
        expect(response.status).toBe(200);
        expect(response.body).toHaveProperty('totalBundles');
      });
    });
  });
});
