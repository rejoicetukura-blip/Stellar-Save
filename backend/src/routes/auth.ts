import { Router, Request, Response } from 'express';
import {
  generateChallenge,
  verifySignature,
  issueJwt,
  issueRefreshToken,
  rotateRefreshToken,
  revokeSession,
  revokeAllSessions,
} from '../auth_service';
import { jwtAuthMiddleware, AuthenticatedRequest } from '../auth_middleware';
import { logger } from '../logger';

/**
 * Auth routes for Stellar wallet-based authentication.
 *
 * POST /api/auth/challenge          — Request sign challenge
 * POST /api/auth/verify             — Verify signature → access + refresh tokens
 * POST /api/auth/refresh            — Rotate refresh token → new token pair
 * POST /api/auth/logout             — Revoke current session family
 * POST /api/auth/logout-everywhere  — Revoke all sessions for the wallet (requires JWT)
 */
export function createAuthRouter(): Router {
  const router = Router();

  router.post('/challenge', async (req: Request, res: Response) => {
    const { walletAddress } = req.body;
    if (!walletAddress || typeof walletAddress !== 'string') {
      return res.status(400).json({ error: 'walletAddress is required' });
    }
    try {
      const challenge = await generateChallenge(walletAddress.trim());
      logger.info('Auth challenge issued', { walletAddress });
      return res.status(200).json({ challenge });
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Failed to generate challenge';
      logger.warn('Auth challenge failed', { walletAddress, error: message });
      return res.status(400).json({ error: message });
    }
  });

  /**
   * POST /api/auth/verify
   * Body: { walletAddress, challenge, signature }
   * Returns: { accessToken, refreshToken }
   */
  router.post('/verify', async (req: Request, res: Response) => {
    const { walletAddress, challenge, signature } = req.body;
    if (!walletAddress || !challenge || !signature) {
      return res.status(400).json({ error: 'walletAddress, challenge, and signature are required' });
    }

    try {
      const isValid = await verifySignature(walletAddress.trim(), challenge, signature);
      if (!isValid) {
        logger.warn('Auth verification failed — invalid signature', { walletAddress });
        return res.status(401).json({ error: 'Invalid signature' });
      }

      const accessToken = issueJwt(walletAddress.trim());
      const refreshToken = await issueRefreshToken(walletAddress.trim());
      logger.info('Auth verification successful', { walletAddress });

      return res.status(200).json({ accessToken, refreshToken });
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Verification failed';
      logger.warn('Auth verify error', { walletAddress, error: message });
      return res.status(401).json({ error: message });
    }
  });

  /**
   * POST /api/auth/refresh
   * Body: { refreshToken }
   * Returns: { accessToken, refreshToken }
   *
   * Rotates the refresh token (one-time-use). Reuse invalidates the whole session family.
   */
  router.post('/refresh', async (req: Request, res: Response) => {
    const { refreshToken } = req.body;
    if (!refreshToken || typeof refreshToken !== 'string') {
      return res.status(400).json({ error: 'refreshToken is required' });
    }

    try {
      const tokens = await rotateRefreshToken(refreshToken);
      return res.status(200).json(tokens);
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Token rotation failed';
      logger.warn('Refresh token rotation failed', { error: message });
      // 401 for all token errors — don't leak internal reason on reuse
      return res.status(401).json({ error: message });
    }
  });

  /**
   * POST /api/auth/logout
   * Body: { refreshToken }
   * Revokes the session family containing this token (no JWT required — token is proof).
   */
  router.post('/logout', async (req: Request, res: Response) => {
    const { refreshToken } = req.body;
    if (!refreshToken || typeof refreshToken !== 'string') {
      return res.status(400).json({ error: 'refreshToken is required' });
    }

    try {
      await revokeSession(refreshToken);
      return res.status(200).json({ message: 'Logged out' });
    } catch (error) {
      logger.warn('Logout error', { error: String(error) });
      return res.status(200).json({ message: 'Logged out' }); // idempotent
    }
  });

  /**
   * POST /api/auth/logout-everywhere
   * Requires: Authorization: Bearer <accessToken>
   * Immediately revokes ALL refresh token sessions for the authenticated wallet.
   */
  router.post('/logout-everywhere', jwtAuthMiddleware, async (req: AuthenticatedRequest, res: Response) => {
    try {
      await revokeAllSessions(req.walletAddress!);
      logger.info('All sessions revoked', { walletAddress: req.walletAddress });
      return res.status(200).json({ message: 'All sessions revoked' });
    } catch (error) {
      logger.error('logout-everywhere error', { error: String(error) });
      return res.status(500).json({ error: 'Failed to revoke sessions' });
    }
  });

  return router;
}
