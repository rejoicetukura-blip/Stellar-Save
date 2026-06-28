/**
 * sw.js — Stellar Save Service Worker
 *
 * Handles:
 *  1. PWA caching:
 *       - cache-first for static assets (stored in a runtime cache, pruned)
 *       - stale-while-revalidate for GET /api/ responses (separate API cache)
 *       - network-first navigations with cached-shell / offline.html fallback
 *  2. Offline fallback page + read-only operation from cached data
 *  3. Cache invalidation on version bumps (purges ALL stale caches)
 *  4. Background push events (Web Push API)
 *  5. Scheduled contribution reminder alarms via postMessage
 *  6. Notification click → focus/open app and navigate to group detail
 */

// ─── Versioning ──────────────────────────────────────────────────────────────
// Bump SW_VERSION on every release so the activate handler purges every cache
// (static, runtime, api) that does not belong to the current version.
const SW_VERSION = 'v2';
const STATIC_CACHE = `stellar-save-static-${SW_VERSION}`;
const RUNTIME_CACHE = `stellar-save-runtime-${SW_VERSION}`;
const API_CACHE = `stellar-save-api-${SW_VERSION}`;
const CURRENT_CACHES = [STATIC_CACHE, RUNTIME_CACHE, API_CACHE];

const STATIC_ASSETS = ['/', '/offline.html', '/manifest.json', '/vite.svg'];
const APP_ORIGIN = self.location.origin;

// Cap the runtime cache so it cannot grow without bound.
const RUNTIME_MAX_ENTRIES = 60;

// ─── Helpers ─────────────────────────────────────────────────────────────────

/** Trim a cache to at most `maxEntries`, evicting oldest entries first (FIFO). */
async function trimCache(cacheName, maxEntries) {
  const cache = await caches.open(cacheName);
  const keys = await cache.keys();
  if (keys.length <= maxEntries) return;
  // keys() preserves insertion order; delete from the front.
  for (let i = 0; i < keys.length - maxEntries; i += 1) {
    await cache.delete(keys[i]);
  }
}

/** Notify all controlled clients that a new SW version is now active. */
async function notifyClientsOfUpdate() {
  const clientList = await self.clients.matchAll({ includeUncontrolled: true, type: 'window' });
  for (const client of clientList) {
    client.postMessage({ type: 'SW_UPDATED', version: SW_VERSION });
  }
}

// ─── Install: pre-cache static shell ─────────────────────────────────────────
self.addEventListener('install', (event) => {
  event.waitUntil(
    caches.open(STATIC_CACHE).then((cache) => cache.addAll(STATIC_ASSETS))
  );
  self.skipWaiting();
});

// ─── Activate: purge ALL stale caches, claim clients, notify of update ────────
self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) =>
        Promise.all(
          keys.filter((k) => !CURRENT_CACHES.includes(k)).map((k) => caches.delete(k))
        )
      )
      .then(() => self.clients.claim())
      .then(() => notifyClientsOfUpdate())
  );
});

// ─── Fetch: caching strategies ───────────────────────────────────────────────
self.addEventListener('fetch', (event) => {
  const { request } = event;
  const url = new URL(request.url);

  // Only handle same-origin GET requests.
  if (request.method !== 'GET' || url.origin !== APP_ORIGIN) return;

  // API calls: stale-while-revalidate (serve cache instantly, refresh in bg).
  if (url.pathname.startsWith('/api/')) {
    event.respondWith(staleWhileRevalidate(event, request));
    return;
  }

  // Navigation requests: network-first, fall back to cached shell or offline page.
  if (request.mode === 'navigate') {
    event.respondWith(
      fetch(request)
        .then((res) => {
          const clone = res.clone();
          caches.open(RUNTIME_CACHE).then((c) => c.put(request, clone));
          return res;
        })
        .catch(async () => {
          // Read-only offline: serve the previously cached route, then the app
          // shell ('/'), then the dedicated offline page as a last resort.
          const cached =
            (await caches.match(request)) ||
            (await caches.match('/')) ||
            (await caches.match('/offline.html'));
          return cached;
        })
    );
    return;
  }

  // Static assets: cache-first, populate the runtime cache and prune it.
  event.respondWith(
    caches.match(request).then(
      (cached) =>
        cached ??
        fetch(request).then((res) => {
          // Only cache successful, basic (same-origin) responses.
          if (res && res.status === 200 && res.type === 'basic') {
            const clone = res.clone();
            caches.open(RUNTIME_CACHE).then((c) => {
              c.put(request, clone).then(() => trimCache(RUNTIME_CACHE, RUNTIME_MAX_ENTRIES));
            });
          }
          return res;
        })
    )
  );
});

// ─── Strategy: stale-while-revalidate for API GETs ───────────────────────────
async function staleWhileRevalidate(event, request) {
  const cache = await caches.open(API_CACHE);
  const cached = await cache.match(request);

  // Kick off a background refresh regardless of cache hit.
  const networkFetch = fetch(request)
    .then((res) => {
      if (res && res.status === 200) {
        cache.put(request, res.clone());
      }
      return res;
    })
    .catch(() => null);

  // Cache hit: return immediately, keep the SW alive for the bg refresh.
  if (cached) {
    event.waitUntil(networkFetch);
    return cached;
  }

  // Cache miss: await the network, fall back to an offline JSON sentinel.
  const networkRes = await networkFetch;
  if (networkRes) return networkRes;

  return new Response(JSON.stringify({ error: 'offline' }), {
    status: 503,
    headers: { 'Content-Type': 'application/json' },
  });
}

// ─── Push Event ──────────────────────────────────────────────────────────────
self.addEventListener('push', (event) => {
  if (!event.data) return;

  let payload;
  try {
    payload = event.data.json();
  } catch {
    payload = { title: 'Contribution Reminder', body: event.data.text(), groupId: null };
  }

  const { title = 'Contribution Reminder', body = 'Your deadline is approaching', groupId } = payload;

  event.waitUntil(
    self.registration.showNotification(title, {
      body,
      icon: '/vite.svg',
      badge: '/vite.svg',
      tag: groupId ? `contribution-${groupId}` : 'contribution-reminder',
      data: { groupId, url: groupId ? `/groups/${groupId}` : '/dashboard' },
      requireInteraction: false,
    })
  );
});

// ─── Message Channel (client-side scheduling) ────────────────────────────────
self.addEventListener('message', (event) => {
  if (!event.data) return;

  if (event.data.type === 'SHOW_NOTIFICATION') {
    const { title, body, groupId } = event.data;
    self.registration.showNotification(title ?? 'Contribution Reminder', {
      body: body ?? 'Your deadline is approaching',
      icon: '/vite.svg',
      badge: '/vite.svg',
      tag: groupId ? `contribution-${groupId}` : 'contribution-reminder',
      data: { groupId, url: groupId ? `/groups/${groupId}` : '/dashboard' },
      requireInteraction: false,
    });
  }

  if (event.data.type === 'SKIP_WAITING') {
    self.skipWaiting();
  }
});

// ─── Notification Click ───────────────────────────────────────────────────────
self.addEventListener('notificationclick', (event) => {
  event.notification.close();

  const targetUrl = event.notification.data?.url ?? '/dashboard';
  const fullUrl = `${APP_ORIGIN}${targetUrl}`;

  event.waitUntil(
    self.clients
      .matchAll({ type: 'window', includeUncontrolled: true })
      .then((clientList) => {
        for (const client of clientList) {
          if (client.url.startsWith(APP_ORIGIN) && 'focus' in client) {
            client.postMessage({ type: 'NAVIGATE', url: targetUrl });
            return client.focus();
          }
        }
        if (self.clients.openWindow) {
          return self.clients.openWindow(fullUrl);
        }
      })
  );
});
