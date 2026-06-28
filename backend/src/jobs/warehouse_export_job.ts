/**
 * Scheduled warehouse export job.
 * Wired up in main.ts / app startup when WAREHOUSE_EXPORT_ENABLED=true.
 */
import { S3Client } from '@aws-sdk/client-s3';
import { WarehouseExportPipeline } from '../warehouse_export';
import { logger } from '../logger';

let timer: ReturnType<typeof setInterval> | null = null;

export function startWarehouseExportJob(opts: {
  intervalMs?: number;
  bucket?: string;
  region?: string;
  alertWebhook?: string;
}): void {
  const bucket = opts.bucket || process.env.BACKUP_S3_BUCKET || 'stellar-save-warehouse';
  const intervalMs = opts.intervalMs ?? 60 * 60 * 1000; // default: hourly

  const s3 = new S3Client({ region: opts.region || process.env.AWS_REGION || 'us-east-1' });
  const pipeline = new WarehouseExportPipeline({
    s3Client: s3,
    bucket,
    alertWebhook: opts.alertWebhook || process.env.BACKUP_ALERT_WEBHOOK_URL,
  });

  const run = async () => {
    try {
      const result = await pipeline.run();
      logger.info('[warehouse-job] Export completed', result);
    } catch (err) {
      logger.error('[warehouse-job] Export failed', err);
    }
  };

  // Run once immediately, then on the interval
  run().catch(() => {});
  timer = setInterval(run, intervalMs);
  logger.info(`[warehouse-job] Scheduled every ${intervalMs / 1000}s → s3://${bucket}`);
}

export function stopWarehouseExportJob(): void {
  if (timer) {
    clearInterval(timer);
    timer = null;
  }
}
