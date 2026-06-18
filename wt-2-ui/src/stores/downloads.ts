import { create } from 'zustand'
import { get, set as setItem } from 'idb-keyval'
import type { DownloadItem, DownloadQuality, AudioFormat } from '../lib/types.ts'

const STORAGE_KEY = 'yt-vanced-downloads'

interface DownloadsState {
  items: DownloadItem[]
  initialized: boolean
}

interface DownloadsActions {
  init: () => Promise<void>
  addDownload: (params: {
    videoId: string
    title: string
    authorName: string
    thumbnailUrl: string
    quality: DownloadQuality
    audioFormat: AudioFormat
    bytesTotal: number
  }) => Promise<void>
  updateProgress: (id: string, progress: number, bytesDownloaded: number) => Promise<void>
  pauseDownload: (id: string) => Promise<void>
  resumeDownload: (id: string) => Promise<void>
  completeDownload: (id: string) => Promise<void>
  failDownload: (id: string, error: string) => Promise<void>
  removeDownload: (id: string) => Promise<void>
}

async function loadItems(): Promise<DownloadItem[]> {
  const raw = await get(STORAGE_KEY)
  if (!raw) return []
  return raw as DownloadItem[]
}

export const useDownloadsStore = create<DownloadsState & DownloadsActions>((set, get) => ({
  items: [],
  initialized: false,
  init: async () => {
    const items = await loadItems()
    set({ items, initialized: true })
  },
  addDownload: async (params) => {
    const item: DownloadItem = {
      id: `${params.videoId}-${Date.now()}`,
      ...params,
      state: 'queued',
      progress: 0,
      bytesDownloaded: 0,
      createdAt: Date.now(),
    }
    const items = [item, ...get().items]
    await setItem(STORAGE_KEY, items)
    set({ items })
  },
  updateProgress: async (id, progress, bytesDownloaded) => {
    const items = get().items.map((item) =>
      item.id === id ? { ...item, progress, bytesDownloaded, state: 'downloading' as const } : item,
    )
    await setItem(STORAGE_KEY, items)
    set({ items })
  },
  pauseDownload: async (id) => {
    const items = get().items.map((item) =>
      item.id === id ? { ...item, state: 'paused' as const } : item,
    )
    await setItem(STORAGE_KEY, items)
    set({ items })
  },
  resumeDownload: async (id) => {
    const items = get().items.map((item) =>
      item.id === id ? { ...item, state: 'downloading' as const } : item,
    )
    await setItem(STORAGE_KEY, items)
    set({ items })
  },
  completeDownload: async (id) => {
    const items = get().items.map((item) =>
      item.id === id
        ? { ...item, state: 'completed' as const, progress: 100, completedAt: Date.now() }
        : item,
    )
    await setItem(STORAGE_KEY, items)
    set({ items })
  },
  failDownload: async (id, error) => {
    const items = get().items.map((item) =>
      item.id === id ? { ...item, state: 'failed' as const, error } : item,
    )
    await setItem(STORAGE_KEY, items)
    set({ items })
  },
  removeDownload: async (id) => {
    const items = get().items.filter((item) => item.id !== id)
    await setItem(STORAGE_KEY, items)
    set({ items })
  },
}))
