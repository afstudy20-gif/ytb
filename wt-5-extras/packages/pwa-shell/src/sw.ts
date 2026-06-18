/// <reference lib="webworker" />

const CACHE_VERSION = 'v1';
const STATIC_CACHE = `static-${CACHE_VERSION}`;
const MEDIA_CACHE = `media-${CACHE_VERSION}`;

const sw = self as unknown as ServiceWorkerGlobalScope;

function getManifest(): string[] {
  const manifest = (sw as unknown as Record<string, unknown>).__WB_MANIFEST;
  if (Array.isArray(manifest) && manifest.every((item): item is string => typeof item === 'string')) {
    return manifest;
  }
  return ['/', '/index.html', '/manifest.json', '/app.js', '/app.css'];
}

/**
 * Determine whether a request looks like a static asset.
 */
function isStatic(request: Request): boolean {
  const dest = request.destination;
  return (
    dest === 'style' ||
    dest === 'script' ||
    dest === 'worker' ||
    dest === 'manifest' ||
    dest === 'document' ||
    request.url.endsWith('.woff2')
  );
}

/**
 * Determine whether a request targets an API endpoint.
 */
function isApi(request: Request): boolean {
  const url = new URL(request.url);
  return url.pathname.startsWith('/api/');
}

/**
 * Determine whether a request is for a media segment.
 */
function isMediaSegment(request: Request): boolean {
  const url = new URL(request.url);
  return (
    url.pathname.includes('/segment') ||
    url.pathname.endsWith('.ts') ||
    url.pathname.endsWith('.m4s')
  );
}

sw.addEventListener('install', (event: ExtendableEvent) => {
  event.waitUntil(
    caches
      .open(STATIC_CACHE)
      .then((cache) => cache.addAll(getManifest()))
      .then(() => sw.skipWaiting()),
  );
});

sw.addEventListener('activate', (event: ExtendableEvent) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) =>
        Promise.all(
          keys
            .filter((key) => key !== STATIC_CACHE && key !== MEDIA_CACHE)
            .map((key) => caches.delete(key)),
        ),
      )
      .then(() => sw.clients.claim()),
  );
});

sw.addEventListener('fetch', (event: FetchEvent) => {
  const { request } = event;

  if (request.method !== 'GET') {
    return;
  }

  if (isStatic(request)) {
    event.respondWith(cacheFirst(request, STATIC_CACHE));
    return;
  }

  if (isApi(request)) {
    event.respondWith(networkFirst(request, STATIC_CACHE));
    return;
  }

  if (isMediaSegment(request)) {
    event.respondWith(cacheFirst(request, MEDIA_CACHE));
  }
});

/**
 * Cache-first strategy: serve from cache, falling back to network.
 * The network response is stored when fresh.
 */
async function cacheFirst(
  request: Request,
  cacheName: string,
): Promise<Response> {
  const cache = await caches.open(cacheName);
  const cached = await cache.match(request);
  if (cached !== undefined) {
    return cached;
  }

  const response = await fetch(request);
  if (response.ok) {
    cache.put(request, response.clone());
  }
  return response;
}

/**
 * Network-first strategy: try network, fall back to cache.
 * Successful network responses are cached.
 */
async function networkFirst(
  request: Request,
  cacheName: string,
): Promise<Response> {
  const cache = await caches.open(cacheName);
  try {
    const response = await fetch(request);
    if (response.ok) {
      cache.put(request, response.clone());
    }
    return response;
  } catch {
    const cached = await cache.match(request);
    if (cached !== undefined) {
      return cached;
    }
    throw new Error(`network offline and no cache for ${request.url}`);
  }
}

export {};
