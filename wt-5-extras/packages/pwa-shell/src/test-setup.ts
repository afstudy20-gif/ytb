import 'fake-indexeddb/auto';

Object.defineProperty(globalThis.navigator, 'mediaSession', {
  value: {
    metadata: null,
    playbackState: 'none',
    setActionHandler: () => undefined,
  },
  writable: true,
  configurable: true,
});

class MockMediaMetadata {
  title?: string;
  artist?: string;
  album?: string;
  artwork?: MediaImage[];

  constructor(meta?: {
    title?: string;
    artist?: string;
    album?: string;
    artwork?: MediaImage[];
  }) {
    if (meta !== undefined) {
      this.title = meta.title;
      this.artist = meta.artist;
      this.album = meta.album;
      this.artwork = meta.artwork;
    }
  }
}

Object.defineProperty(globalThis, 'MediaMetadata', {
  value: MockMediaMetadata,
  writable: true,
  configurable: true,
});

function readBlob(blob: Blob, method: 'readAsArrayBuffer' | 'readAsText'): Promise<string | ArrayBuffer> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(reader.result ?? (method === 'readAsArrayBuffer' ? new ArrayBuffer(0) : ''));
    reader.onerror = () => reject(reader.error);
    if (method === 'readAsArrayBuffer') {
      reader.readAsArrayBuffer(blob);
    } else {
      reader.readAsText(blob);
    }
  });
}

if (!Blob.prototype.arrayBuffer) {
  Object.defineProperty(Blob.prototype, 'arrayBuffer', {
    value: async function arrayBuffer(): Promise<ArrayBuffer> {
      const result = await readBlob(this, 'readAsArrayBuffer');
      return result as ArrayBuffer;
    },
    writable: true,
    configurable: true,
  });
}

if (!Blob.prototype.text) {
  Object.defineProperty(Blob.prototype, 'text', {
    value: async function text(): Promise<string> {
      const result = await readBlob(this, 'readAsText');
      return result as string;
    },
    writable: true,
    configurable: true,
  });
}
