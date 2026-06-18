import { create } from 'zustand'
import type { VideoDetail, VideoSummary } from '../lib/types.ts'

type QueueItem = VideoSummary | VideoDetail

interface PlayerState {
  queue: QueueItem[]
  currentIndex: number
  isPlaying: boolean
  isExpanded: boolean
  isAudioOnly: boolean
  backgroundAudio: boolean
  playbackRate: number
  volume: number
  currentTime: number
  selectedQuality: string | null
}

interface PlayerActions {
  playVideo: (video: QueueItem) => void
  playQueue: (videos: QueueItem[], startIndex?: number) => void
  togglePlay: () => void
  setPlaying: (playing: boolean) => void
  expand: () => void
  collapse: () => void
  next: () => void
  previous: () => void
  setAudioOnly: (audioOnly: boolean) => void
  setBackgroundAudio: (enabled: boolean) => void
  setPlaybackRate: (rate: number) => void
  setVolume: (volume: number) => void
  setCurrentTime: (time: number) => void
  seekBy: (deltaSeconds: number) => void
  setSelectedQuality: (quality: string | null) => void
  close: () => void
}

const initialState: PlayerState = {
  queue: [],
  currentIndex: 0,
  isPlaying: false,
  isExpanded: false,
  isAudioOnly: false,
  backgroundAudio: false,
  playbackRate: 1,
  volume: 1,
  currentTime: 0,
  selectedQuality: null,
}

export const usePlayerStore = create<PlayerState & PlayerActions>((set, get) => ({
  ...initialState,
  playVideo: (video) =>
    set({
      queue: [video],
      currentIndex: 0,
      isPlaying: true,
      isExpanded: true,
      currentTime: 0,
      selectedQuality: null,
    }),
  playQueue: (videos, startIndex = 0) =>
    set({
      queue: videos,
      currentIndex: startIndex,
      isPlaying: true,
      isExpanded: true,
      currentTime: 0,
      selectedQuality: null,
    }),
  togglePlay: () => set((state) => ({ isPlaying: !state.isPlaying })),
  setPlaying: (playing) => set({ isPlaying: playing }),
  expand: () => set({ isExpanded: true }),
  collapse: () => set({ isExpanded: false }),
  next: () => {
    const { queue, currentIndex } = get()
    if (currentIndex < queue.length - 1) {
      set({ currentIndex: currentIndex + 1, currentTime: 0 })
    }
  },
  previous: () => {
    const { currentIndex } = get()
    if (currentIndex > 0) {
      set({ currentIndex: currentIndex - 1, currentTime: 0 })
    }
  },
  setAudioOnly: (audioOnly) => set({ isAudioOnly: audioOnly }),
  setBackgroundAudio: (enabled) => set({ backgroundAudio: enabled }),
  setPlaybackRate: (rate) => set({ playbackRate: rate }),
  setVolume: (volume) => set({ volume: Math.max(0, Math.min(1, volume)) }),
  setCurrentTime: (time) => set({ currentTime: time }),
  seekBy: (deltaSeconds) =>
    set((state) => ({
      currentTime: Math.max(0, state.currentTime + deltaSeconds),
    })),
  setSelectedQuality: (quality) => set({ selectedQuality: quality }),
  close: () => set({ ...initialState }),
}))
