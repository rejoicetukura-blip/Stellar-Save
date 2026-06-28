/**
 * routes/ramp.ts
 *
 * Fiat on/off-ramp and KYC endpoint stubs.
 * Each route applies the full ramp protection stack:
 *   - strict per-IP rate limit (5 req / 15 min)
 *   - CAPTCHA gate for unauthenticated callers
 *   - per-user deposit velocity limit (3 deposits / hour)
 */

import { Router, Request, Response } from 'express';
import { rampProtection } from '../fiat_ramp_protection';
import { logger } from '../logger';

export function createRampRouter(): Router {
  const router = Router();

  // POST /api/ramp/deposit
  // Initiate a fiat-to-XLM deposit via anchor.
  router.post(
    '/deposit',
    ...rampProtection({ velocityCheck: true }),
    (req: Request, res: Response) => {
      // TODO: integrate with SEP-6 / SEP-24 anchor SDK
      logger.info('[ramp] deposit initiated', { ip: req.ip });
      res.status(202).json({ status: 'pending', message: 'Deposit initiation received' });
    },
  );

  // POST /api/ramp/withdraw
  // Initiate an XLM-to-fiat withdrawal.
  router.post(
    '/withdraw',
    ...rampProtection({ velocityCheck: false }),
    (req: Request, res: Response) => {
      logger.info('[ramp] withdrawal initiated', { ip: req.ip });
      res.status(202).json({ status: 'pending', message: 'Withdrawal initiation received' });
    },
  );

  // POST /api/ramp/kyc
  // Submit KYC documents for an anchor.
  router.post(
    '/kyc',
    ...rampProtection({ velocityCheck: false }),
    (req: Request, res: Response) => {
      logger.info('[ramp] KYC submission', { ip: req.ip });
      res.status(202).json({ status: 'pending', message: 'KYC submission received' });
    },
  );

  // GET /api/ramp/status/:transactionId
  // Poll transaction status. IP-limited but no velocity check.
  router.get(
    '/status/:transactionId',
    ...rampProtection({ velocityCheck: false }),
    (req: Request, res: Response) => {
      const { transactionId } = req.params;
      res.json({ transactionId, status: 'pending' });
    },
  );

  return router;
}
