export interface NowPlayingMeta {
  /** Track title. */
  title: string;
  /** Track artist. */
  artist?: string;
  /** Album name. */
  album?: string;
  /** Artwork sources. */
  artwork?: MediaImage[];
}

export type MediaSessionAction =
  | 'play'
  | 'pause'
  | 'previoustrack'
  | 'nexttrack'
  | 'seekbackward'
  | 'seekforward'
  | 'seekto'
  | 'stop';

export interface BackgroundAudioController {
  /** Attach an `<audio>` or `<video>` element and update MediaSession metadata. */
  attach(audio: HTMLMediaElement, meta: NowPlayingMeta): void;
  /** Detach the current media element and clear handlers. */
  detach(): void;
  /** Update MediaSession metadata without re-attaching. */
  setMeta(meta: NowPlayingMeta): void;
  /** Update the platform playback state. */
  setPlaybackState(state: 'playing' | 'paused' | 'none'): void;
  /** Register a handler for a MediaSession action. */
  onAction(
    action: MediaSessionAction,
    handler: (detail?: MediaSessionActionDetails) => void,
  ): void;
}

function setMetadata(meta: NowPlayingMeta): void {
  if ('mediaSession' in navigator) {
    navigator.mediaSession.metadata = new MediaMetadata({
      title: meta.title,
      artist: meta.artist,
      album: meta.album,
      artwork: meta.artwork,
    });
  }
}

/**
 * Create a controller that bridges an HTML media element to the platform
 * MediaSession API.
 */
export function createBackgroundAudioController(): BackgroundAudioController {
  let current: HTMLMediaElement | undefined;
  const handlers = new Map<
    MediaSessionAction,
    (detail?: MediaSessionActionDetails) => void
  >();

  const controller: BackgroundAudioController = {
    attach(audio, meta) {
      detachCurrent();
      current = audio;
      setMetadata(meta);
      setPlaybackHandlers(audio);
    },

    detach() {
      detachCurrent();
    },

    setMeta(meta) {
      setMetadata(meta);
    },

    setPlaybackState(state) {
      if ('mediaSession' in navigator) {
        navigator.mediaSession.playbackState = state;
      }
    },

    onAction(action, handler) {
      handlers.set(action, handler);
      if ('mediaSession' in navigator) {
        navigator.mediaSession.setActionHandler(action, (details) => {
          handler(details);
        });
      }
    },
  };

  function detachCurrent(): void {
    if (current === undefined) {
      return;
    }
    current = undefined;
    if ('mediaSession' in navigator) {
      navigator.mediaSession.metadata = null;
      navigator.mediaSession.playbackState = 'none';
    }
  }

  function setPlaybackHandlers(audio: HTMLMediaElement): void {
    const playHandler = handlers.get('play');
    if (playHandler !== undefined) {
      if ('mediaSession' in navigator) {
        navigator.mediaSession.setActionHandler('play', () => {
          void audio.play();
          playHandler();
        });
      }
    }
    const pauseHandler = handlers.get('pause');
    if (pauseHandler !== undefined) {
      if ('mediaSession' in navigator) {
        navigator.mediaSession.setActionHandler('pause', () => {
          audio.pause();
          pauseHandler();
        });
      }
    }
  }

  return controller;
}
