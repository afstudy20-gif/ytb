import { describe, it, expect } from 'vitest';
import { openOfflineStore } from './index.js';

describe('openOfflineStore', () => {
  it('stores and retrieves a blob', async () => {
    const store = await openOfflineStore('test-store-1');
    const blob = new Blob(['hello'], { type: 'text/plain' });
    await store.putBlob('greeting', blob, { title: 'hello' });

    const result = await store.getBlob('greeting');
    expect(result).toBeDefined();
    expect(result?.meta).toEqual({ title: 'hello' });
    const text = await (result?.blob as Blob).text();
    expect(text).toBe('hello');
  });

  it('lists entries with size', async () => {
    const store = await openOfflineStore('test-store-2');
    await store.putBlob('a', new Blob(['x']), {});
    await store.putBlob('b', new Blob(['yy']), {});

    const entries = await store.list();
    expect(entries).toHaveLength(2);
    expect(entries.reduce((sum, entry) => sum + entry.size, 0)).toBe(3);
    expect(await store.totalSize()).toBe(3);
  });

  it('deletes an entry', async () => {
    const store = await openOfflineStore('test-store-3');
    await store.putBlob('temp', new Blob(['z']), {});
    await store.delete('temp');
    expect(await store.getBlob('temp')).toBeUndefined();
  });
});
