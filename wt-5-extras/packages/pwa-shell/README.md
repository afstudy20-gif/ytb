# @wt-5/pwa-shell

PWA helpers for wt-5: service worker registration, background audio via
MediaSession, and an IndexedDB offline blob store.

```ts
import { registerServiceWorker, createBackgroundAudioController } from '@wt-5/pwa-shell';

await registerServiceWorker({ url: '/sw.js' });

const audio = document.querySelector('audio');
const controller = createBackgroundAudioController();
controller.attach(audio, { title: 'Track', artist: 'Artist' });
```

Ship the included service worker from `@wt-5/pwa-shell/sw`.
