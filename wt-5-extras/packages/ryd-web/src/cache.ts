import type { Votes } from './model.js';

interface CacheEntry {
  votes: Votes;
  expiresAt: number;
}

export interface LruOptions {
  capacity: number;
  ttlMs: number;
}

/**
 * Tiny bounded LRU cache with per-entry TTL.
 */
export class LruCache {
  private readonly capacity: number;
  private readonly ttlMs: number;
  private readonly order: string[] = [];
  private readonly entries = new Map<string, CacheEntry>();

  constructor(options: LruOptions) {
    this.capacity = Math.max(1, options.capacity);
    this.ttlMs = options.ttlMs;
  }

  /**
   * Look up a key, returning `undefined` when missing or stale.
   * A hit promotes the entry to most-recently-used.
   */
  get(key: string): Votes | undefined {
    const entry = this.entries.get(key);
    if (entry === undefined) {
      return undefined;
    }
    if (Date.now() >= entry.expiresAt) {
      this.delete(key);
      return undefined;
    }
    this.touch(key);
    return entry.votes;
  }

  /**
   * Insert or replace an entry, evicting the least-recently-used element
   * when over capacity.
   */
  put(key: string, votes: Votes): void {
    if (this.entries.has(key)) {
      this.entries.set(key, { votes, expiresAt: Date.now() + this.ttlMs });
      this.touch(key);
      return;
    }

    while (this.order.length >= this.capacity) {
      const victim = this.order.shift();
      if (victim === undefined) break;
      this.entries.delete(victim);
    }

    this.entries.set(key, { votes, expiresAt: Date.now() + this.ttlMs });
    this.order.push(key);
  }

  /**
   * Number of entries currently stored.
   */
  size(): number {
    return this.entries.size;
  }

  private touch(key: string): void {
    const index = this.order.indexOf(key);
    if (index > -1) {
      this.order.splice(index, 1);
      this.order.push(key);
    }
  }

  private delete(key: string): void {
    const index = this.order.indexOf(key);
    if (index > -1) {
      this.order.splice(index, 1);
    }
    this.entries.delete(key);
  }
}
