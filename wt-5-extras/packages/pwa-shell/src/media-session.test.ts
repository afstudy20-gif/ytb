import { describe, it, expect, vi } from 'vitest';
import { createBackgroundAudioController } from './index.js';

describe('createBackgroundAudioController', () => {
  it('sets metadata on attach', () => {
    const controller = createBackgroundAudioController();
    const audio = document.createElement('audio');
    controller.attach(audio, {
      title: 'Song',
      artist: 'Artist',
      album: 'Album',
    });

    expect(navigator.mediaSession.metadata?.title).toBe('Song');
    expect(navigator.mediaSession.metadata?.artist).toBe('Artist');
    expect(navigator.mediaSession.metadata?.album).toBe('Album');
  });

  it('clears metadata on detach', () => {
    const controller = createBackgroundAudioController();
    const audio = document.createElement('audio');
    controller.attach(audio, { title: 'Song' });
    controller.detach();

    expect(navigator.mediaSession.metadata).toBeNull();
  });

  it('updates playback state', () => {
    const controller = createBackgroundAudioController();
    controller.setPlaybackState('playing');
    expect(navigator.mediaSession.playbackState).toBe('playing');
  });

  it('registers action handlers', () => {
    const controller = createBackgroundAudioController();
    const handler = vi.fn();
    controller.onAction('nexttrack', handler);
    expect(typeof navigator.mediaSession.setActionHandler).toBe('function');
  });
});
