// ── Distributed tracing ───────────────────────────────────────────────────────
// MUST be the very first import so OpenTelemetry can patch instrumented libraries
// (express, http, pg, ioredis, …) before they are required. No-op when tracing
// is disabled (the default).
import { startTracing } from './tracing';
startTracing();

import fs from 'fs';
import http2 from 'http2';
import dotenv from 'dotenv';

dotenv.config();

import express from 'express';
import compression from 'compression';
import cors from 'cors';
import { ApolloServer } from '@apollo/server';
import { expressMiddleware } from '@apollo/server/express4';
import { makeExecutableSchema } from '@graphql-tools/schema';
import { RecommendationEngine } from './recommendation';
import { ABTestingFramework } from './ab_testing';
import { Group, UserInteraction } from './models';
import { EmailService } from './email_service';
import { ExportService } from './export_service';
import { BackupService, S3HttpClient } from './backup_service';
import { BackupScheduler } from './backup_scheduler';
import { RecoveryService } from './recovery_service';
import { BackupMonitor } from './backup_monitor';
import { ContractEventIndexer } from './contract_event_indexer';
import { WebPushService } from './web_push_service';
import { versionMiddleware } from './versioning';
import { createV1Router } from './routes/v1';
import { FeedbackService } from './feedback_service';
import { createV2Router } from './routes/v2';
import { metricsMiddleware, metricsHandler } from './metrics';
import { requestLogger } from './logger';
import { disconnectPrisma } from './prisma_client';
import { createRateLimiterMiddleware, createAuthRateLimiterMiddleware } from './rate_limiter';
import { createTieredRateLimiter, configureTier, setEndpointCost } from './redis_rate_limiter';
import { createQuotaReporterRouter } from './routes/quota_reporter';
import { createWebhookRouter } from './routes/webhooks';
import { getMemberReputation } from './reputation_service';
import { createAuthRouter } from './routes/auth';
import { createUserRouter } from './routes/user';
import { createRampRouter } from './routes/ramp';
import { errorMiddleware, notFoundMiddleware } from './lib/errorMiddleware';

const CSP_POLICY = [
  "default-src 'self'",
  "script-src 'self' https://cdn.jsdelivr.net/npm/stellar-sdk",
  "style-src 'self' 'unsafe-inline'",
  "img-src 'self' data:",
  "connect-src 'self' https://horizon-testnet.stellar.org https://soroban-testnet.stellar.org https://horizon.stellar.org",
  "font-src 'self'",
  "object-src 'none'",
  "base-uri 'self'",
  "form-action 'self'",
  "frame-ancestors 'none'",
  "report-uri /api/csp-report",
].join('; ');

const app = express();
app.use(cors());
app.use(express.json());
app.use(compression());
app.use(requestLogger);
app.use(metricsMiddleware);

// CSP middleware — applied to all responses
app.use((_req, res, next) => {
  res.setHeader('Content-Security-Policy', CSP_POLICY);
  next();
});

configureTier('free', [
  { windowMs: 60_000, max: config.rateLimiting.free.perMin, label: '1m' },
  { windowMs: 3_600_000, max: config.rateLimiting.free.perHour, label: '1h' },
]);
configureTier('pro', [
  { windowMs: 60_000, max: config.rateLimiting.pro.perMin, label: '1m' },
  { windowMs: 3_600_000, max: config.rateLimiting.pro.perHour, label: '1h' },
]);
configureTier('enterprise', [
  { windowMs: 60_000, max: config.rateLimiting.enterprise.perMin, label: '1m' },
  { windowMs: 3_600_000, max: config.rateLimiting.enterprise.perHour, label: '1h' },
]);

setEndpointCost('/api/v1/health', 1, 'read');
setEndpointCost('/api/v1/ready', 1, 'read');
setEndpointCost('/api/v1/stats', 1, 'read');
setEndpointCost('/api/v1/search', 5, 'read');
setEndpointCost('/api/v1/export', 10, 'write');
setEndpointCost('/api/v1/analytics', 5, 'read');
setEndpointCost('/api/ramp/deposit', 10, 'sensitive');
setEndpointCost('/api/ramp/initiate', 10, 'sensitive');
setEndpointCost('/api/kyc/submit', 10, 'sensitive');
setEndpointCost('/api/admin', 5, 'admin');
setEndpointCost('/graphql', 2, 'read');

app.get('/metrics', metricsHandler);
app.use(createTieredRateLimiter());

// Stricter rate limiting on auth/admin endpoints: 10 req / 15 min per IP
const authRateLimiter = createAuthRateLimiterMiddleware();
app.use('/api/admin', authRateLimiter);
app.use('/graphql', authRateLimiter);

// ── CSP violation reporting ───────────────────────────────────────────────────
app.post('/api/csp-report', express.json({ type: ['application/json', 'application/csp-report'] }), (req, res) => {
  const report = req.body?.['csp-report'] ?? req.body;
  console.warn('[CSP Violation]', JSON.stringify(report));
  res.status(204).end();
});

// ========== CACHE ROUTES (Issue #563) ==========

// Cache statistics endpoint - monitor cache hit rates
app.get('/api/cache/stats', async (req, res) => {
  const stats = await getCacheStats();
  res.json(stats);
});

// Example cached endpoint for retirements
app.get('/api/retirements', cacheMiddleware(60), async (req, res) => {
  res.json({ 
    data: 'Retirements data - cached for 60 seconds', 
    timestamp: new Date(),
    source: 'database'
  });
});

// Write endpoint that invalidates cache
app.post('/api/retirements', async (req, res) => {
  await clearCache('/api/retirements');
  res.json({ 
    success: true, 
    message: 'Retirement created, cache cleared',
    timestamp: new Date()
  });
});

// Cached stats endpoint
app.get('/api/stats', cacheMiddleware(3600), async (req, res) => {
  res.json({
    totalRetired: 1000,
    totalTransactions: 45,
    timestamp: new Date(),
    source: 'database'
  });
});

// Start cache warming job (preloads popular data)
startWarmingJob();

// ── GraphQL ───────────────────────────────────────────────────────────────────
const schema = makeExecutableSchema({ typeDefs, resolvers });
const apolloServer = new ApolloServer({
  schema,
  validationRules,
  introspection: true,
});

// Apollo must be started before attaching middleware
apolloServer.start().then(() => {
  // Playground: GET /graphql returns Apollo Sandbox redirect
  app.get('/graphql', (_req, res) => {
    res.send(`
      <!DOCTYPE html><html><head><title>GraphQL Playground</title></head><body>
      <script>window.location.href = 'https://studio.apollographql.com/sandbox/explorer?endpoint=' + encodeURIComponent(window.location.origin + '/graphql');</script>
      </body></html>
    `);
  });

  app.use('/graphql', expressMiddleware(apolloServer, {
    context: async () => ({}),
  }));
});

const PORT = config.port;

// ── Mock Data ────────────────────────────────────────────────────────────────
const mockGroups: Group[] = [
  { id: '1', name: 'Weekly Savers', contributionAmount: 100, cycleDuration: 604800, maxMembers: 10, currentMembers: 5, status: 'Active', tags: ['weekly', 'low-entry'] },
  { id: '2', name: 'Monthly Builders', contributionAmount: 1000, cycleDuration: 2592000, maxMembers: 12, currentMembers: 3, status: 'Active', tags: ['monthly', 'high-entry'] },
  { id: '3', name: 'Student Circle', contributionAmount: 50, cycleDuration: 604800, maxMembers: 5, currentMembers: 4, status: 'Active', tags: ['weekly', 'students'] },
];

const mockInteractions: UserInteraction[] = [
  { userId: 'user1', groupId: '1', interactionType: 'join', timestamp: Date.now() },
  { userId: 'user1', groupId: '2', interactionType: 'join', timestamp: Date.now() },
  { userId: 'user2', groupId: '1', interactionType: 'join', timestamp: Date.now() },
];

// ── Services ─────────────────────────────────────────────────────────────────
const engine = new RecommendationEngine(mockGroups, mockInteractions);
const abTest = new ABTestingFramework();
const emailService = new EmailService();
const exportService = new ExportService(emailService, engine.getInteractions(), engine.getPreferences());
const s3Client = new S3HttpClient();
const backupService = new BackupService(s3Client);
const backupScheduler = new BackupScheduler(backupService);
const recoveryService = new RecoveryService(backupService, s3Client);
const backupMonitor = new BackupMonitor(backupService, {
  alertWebhookUrl: config.backup.alertWebhookUrl,
});

const adminService = new AdminService();

const webPushService = new WebPushService();

const eventIndexer = new ContractEventIndexer(
  process.env.HORIZON_URL || 'https://horizon-testnet.stellar.org',
  process.env.CONTRACT_ID || 'CA...', // Placeholder contract ID
  process.env.DATABASE_URL || 'postgresql://user:pass@localhost:5432/stellar_save',
  webPushService
);

if (process.env.BACKUP_ENABLED === 'true') {
  backupScheduler.start();
  backupMonitor.start();
}

// Start the contract event indexer
if (process.env.INDEXER_ENABLED === 'true') {
  eventIndexer.start().catch(console.error);
}

// Start on-chain anomaly monitor
if (process.env.ON_CHAIN_MONITOR_ENABLED === 'true') {
  const onChainMonitor = new OnChainMonitor({
    largePayoutThresholdStroops: BigInt(
      process.env.ON_CHAIN_LARGE_PAYOUT_THRESHOLD_STROOPS ?? '100000000000'
    ),
  });
  onChainMonitor.start();
}

// Start analytics resync job if enabled
if (process.env.ANALYTICS_RESYNC_ENABLED === 'true') {
  startAnalyticsResyncJob(process.env.ANALYTICS_RESYNC_SCHEDULE || '0 * * * *'); // default: top of every hour
}

// Start keeper/relayer for automated payout execution (Issue #1026)
if (config.keeper.enabled) {
  startKeeperJob(config.keeper.schedule, process.env.CONTRACT_ID || '', config.stellar.rpcUrl);
}

const services = { engine, abTest, exportService, backupService, backupScheduler, recoveryService, backupMonitor, eventIndexer };

// ── Auth routes (public — no JWT required) ───────────────────────────────────
app.use('/api/auth', createAuthRouter());

// ── User routes (JWT protected) ───────────────────────────────────────────────
app.use('/api/user', createUserRouter());

// ── KYC routes (Issue #1024) ──────────────────────────────────────────────────
app.use('/api/kyc', createKycRouter());

// ── Fiat ramp routes (Issue #1023) ────────────────────────────────────────────
app.use('/api/ramp', createRampRouter());

// ── SEP-31 cross-border routes (Issue #1025) ──────────────────────────────────
app.use('/api/sep31', createSep31Router());

// ── Versioned API routes ──────────────────────────────────────────────────────
app.use('/api', versionMiddleware);
app.use('/api/v1', createV1Router(services));
app.use('/api/v2', createV2Router(services));
app.use('/api/webhooks', createWebhookRouter());
app.use('/api/v1/costs', createCostRouter());
app.use('/api/v1/rate-limits', createQuotaReporterRouter());

// ── Fiat ramp routes (strict rate limiting + CAPTCHA gate) ────────────────────
app.use('/api/ramp', createRampRouter());

// ── Member reputation endpoint (Issue #800) ───────────────────────────────────
app.get('/api/members/:address/reputation', async (req, res) => {
  const { address } = req.params;
  if (!address || address.trim().length === 0) {
    return res.status(400).json({ error: 'address is required' });
  }
  try {
    const reputation = await getMemberReputation(address.trim());
    return res.json(reputation);
  } catch {
    return res.status(500).json({ error: 'Failed to fetch reputation' });
  }
});

// ── Legacy unversioned routes (redirect to v1 for backward compatibility) ────
app.use((req, res, next) => {
  const legacyPaths = ['/health', '/recommendations', '/preferences', '/export', '/backup', '/search'];
  if (legacyPaths.some(p => req.path.startsWith(p))) {
    res.setHeader('X-API-Deprecation-Notice', 'Unversioned paths are deprecated. Use /api/v1/...');
    res.setHeader('Deprecation', 'true');
    res.setHeader('Sunset', '2027-01-01');
  }
  next();
});
app.use('/', createV1Router(services));

// ── Error handling (must be last) ─────────────────────────────────────────────
app.use(notFoundMiddleware);
app.use(errorMiddleware);

const hasTls = Boolean(process.env.TLS_KEY_PATH && process.env.TLS_CERT_PATH);
const server = hasTls
  ? http2.createSecureServer(
      {
        key: fs.readFileSync(process.env.TLS_KEY_PATH as string),
        cert: fs.readFileSync(process.env.TLS_CERT_PATH as string),
        allowHTTP1: true,
      },
      app
    )
  : http2.createServer({ allowHTTP1: true }, app);

server.listen(PORT, async () => {
  console.log(`API server running on port ${PORT}`);
  console.log(`  HTTP/2 enabled${hasTls ? ' (TLS)' : ' (h2c cleartext)'}.`);
  console.log(`  Versioned:  /api/v1/...  /api/v2/...`);
  console.log(`  Legacy:     /health  /recommendations  etc. (deprecated)`);
  console.log(`  Cache stats: http://localhost:${PORT}/api/cache/stats`);

  // Start fraud detection worker (Issue #1028)
  if (process.env.FRAUD_DETECTION_ENABLED !== 'false') {
    await fraudDetectionWorker.start();
  }
});

// Graceful shutdown
process.on('SIGTERM', () => {
  fraudDetectionWorker.stop();
  server.close();
  disconnectPrisma().catch(() => {});
});

export { app }; 