import { openDB, type DBSchema, type IDBPDatabase } from 'idb';

const DEFAULT_STORE_NAME = 'wt5-offline';
const DB_VERSION = 1;

interface StoredBlob {
  key: string;
  buffer: ArrayBuffer;
  type: string;
  meta: Record<string, unknown>;
  size: number;
}

interface OfflineStoreSchema extends DBSchema {
  blobs: {
    key: string;
    value: StoredBlob;
  };
}

export interface OfflineStore {
  /** Store a blob with associated metadata. */
  putBlob(
    key: string,
    blob: Blob,
    meta: Record<string, unknown>,
  ): Promise<void>;
  /** Retrieve a previously stored blob and its metadata. */
  getBlob(key: string): Promise<{ blob: Blob; meta: Record<string, unknown> } | undefined>;
  /** Delete a stored entry. */
  delete(key: string): Promise<void>;
  /** List all stored entries with their sizes and metadata. */
  list(): Promise<{ key: string; size: number; meta: Record<string, unknown> }[]>;
  /** Sum of stored blob sizes in bytes. */
  totalSize(): Promise<number>;
}

/**
 * Open an IndexedDB-backed offline blob store.
 *
 * @param name - optional store/database name prefix
 * @returns offline store implementation
 */
export async function openOfflineStore(
  name: string = DEFAULT_STORE_NAME,
): Promise<OfflineStore> {
  const db = await openDB<OfflineStoreSchema>(name, DB_VERSION, {
    upgrade(database) {
      database.createObjectStore('blobs', { keyPath: 'key' });
    },
  });

  return new IdbOfflineStore(db);
}

class IdbOfflineStore implements OfflineStore {
  constructor(private readonly db: IDBPDatabase<OfflineStoreSchema>) {}

  async putBlob(
    key: string,
    blob: Blob,
    meta: Record<string, unknown>,
  ): Promise<void> {
    const buffer = await blob.arrayBuffer();
    await this.db.put('blobs', {
      key,
      buffer,
      type: blob.type,
      meta,
      size: blob.size,
    });
  }

  async getBlob(
    key: string,
  ): Promise<{ blob: Blob; meta: Record<string, unknown> } | undefined> {
    const record = await this.db.get('blobs', key);
    if (record === undefined) {
      return undefined;
    }
    return { blob: new Blob([record.buffer], { type: record.type }), meta: record.meta };
  }

  async delete(key: string): Promise<void> {
    await this.db.delete('blobs', key);
  }

  async list(): Promise<
    { key: string; size: number; meta: Record<string, unknown> }[]
  > {
    const records = await this.db.getAll('blobs');
    return records.map((record) => ({
      key: record.key,
      size: record.size,
      meta: record.meta,
    }));
  }

  async totalSize(): Promise<number> {
    const records = await this.db.getAll('blobs');
    return records.reduce((sum, record) => sum + record.size, 0);
  }
}
