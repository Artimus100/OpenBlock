// Setup file for Jest tests
jest.setTimeout(30000); // 30 second timeout for tests

// Mock external dependencies
jest.mock('redis', () => ({
  createClient: jest.fn(() => ({
    connect: jest.fn(),
    disconnect: jest.fn(),
    set: jest.fn(),
    get: jest.fn(),
    del: jest.fn(),
    exists: jest.fn(),
    expire: jest.fn(),
  })),
}));

// Global test setup
beforeAll(async () => {
  // Setup test database or any global resources
});

afterAll(async () => {
  // Cleanup test resources
});

beforeEach(() => {
  // Reset mocks before each test
  jest.clearAllMocks();
});
