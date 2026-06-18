import { RydError } from './error.js';
import type { Votes } from './model.js';
import { LruCache } from './cache.js';

const DEFAULT_BASE = 'https://returnyoutubedislike.com';
const DEFAULT_CACHE_SIZE = 256;
const DEFAULT_TTL_MS = 5 * 60 * 1000;

function isSuccess(status: number): boolean {
  return status >= 200 && status < 300;
}

function mapStatus(status: number): RydError {
  switch (status) {
    case 400:
      return new RydError('invalidInput', `HTTP ${status}`);
    case 404:
      return new RydError('notFound', 'video has no dislike record');
    case 429:
      return new RydError('rateLimited', `HTTP ${status}`);
    default:
      return new RydError('network', `HTTP ${status}`);
  }
}

function requireVideoId(videoId: string): void {
  if (videoId.trim().length === 0) {
    throw new RydError('invalidInput', 'videoId must not be empty');
  }
}

export interface RydClientOptions {
  /** Base URL for the Return YouTube Dislike API. */
  base?: string;
  /** Maximum number of cached entries (default 256). */
  cacheSize?: number;
  /** Cache time-to-live in milliseconds (default 5 minutes). */
  ttlMs?: number;
}

/**
 * Async fetch-based client for the Return YouTube Dislike API with an
 * in-memory LRU cache.
 */
export class RydClient {
  private readonly base: string;
  private readonly cache: LruCache;

  constructor(options: RydClientOptions = {}) {
    this.base = options.base ?? DEFAULT_BASE;
    this.cache = new LruCache({
      capacity: options.cacheSize ?? DEFAULT_CACHE_SIZE,
      ttlMs: options.ttlMs ?? DEFAULT_TTL_MS,
    });
  }

  /**
   * Fetch like/dislike snapshot for `videoId`, served from cache when fresh.
   *
   * @param videoId - YouTube video id
   * @returns vote snapshot
   */
  async votes(videoId: string): Promise<Votes> {
    requireVideoId(videoId);
    const cached = this.cache.get(videoId);
    if (cached !== undefined) {
      return cached;
    }

    const params = new URLSearchParams({ videoId });
    let response: Response;
    try {
      response = await fetch(`${this.base}/votes?${params.toString()}`);
    } catch (cause) {
      const message = cause instanceof Error ? cause.message : String(cause);
      throw new RydError('network', message);
    }

    if (!isSuccess(response.status)) {
      throw mapStatus(response.status);
    }

    const body = await response.text();
    let parsed: Votes;
    try {
      parsed = JSON.parse(body) as Votes;
    } catch (cause) {
      const message = cause instanceof Error ? cause.message : String(cause);
      throw new RydError('decode', message);
    }

    if (!parsed.deleted) {
      this.cache.put(videoId, parsed);
    }
    return parsed;
  }
}
