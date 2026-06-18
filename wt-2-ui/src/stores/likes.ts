import { create } from 'zustand'
import { get, set as setItem } from 'idb-keyval'
import type { LikedVideo } from '../lib/types.ts'

const STORAGE_KEY = 'yt-vanced-likes'

interface LikesState {
  videos: LikedVideo[]
  initialized: boolean
}

interface LikesActions {
  init: () => Promise<void>
  like: (video: Omit<LikedVideo, 'likedAt'>) => Promise<void>
  unlike: (videoId: string) => Promise<void>
  isLiked: (videoId: string) => boolean
  toggle: (video: Omit<LikedVideo, 'likedAt'>) => Promise<void>
}

async function loadVideos(): Promise<LikedVideo[]> {
  const raw = await get(STORAGE_KEY)
  if (!raw) return []
  return raw as LikedVideo[]
}

export const useLikesStore = create<LikesState & LikesActions>((set, get) => ({
  videos: [],
  initialized: false,
  init: async () => {
    const videos = await loadVideos()
    set({ videos, initialized: true })
  },
  like: async (video) => {
    const videos = [{ ...video, likedAt: Date.now() }, ...get().videos.filter((v) => v.videoId !== video.videoId)]
    await setItem(STORAGE_KEY, videos)
    set({ videos })
  },
  unlike: async (videoId) => {
    const videos = get().videos.filter((v) => v.videoId !== videoId)
    await setItem(STORAGE_KEY, videos)
    set({ videos })
  },
  isLiked: (videoId) => get().videos.some((v) => v.videoId === videoId),
  toggle: async (video) => {
    if (get().isLiked(video.videoId)) {
      await get().unlike(video.videoId)
    } else {
      await get().like(video)
    }
  },
}))
