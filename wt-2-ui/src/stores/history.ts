import { create } from 'zustand'
import { get, set as setItem } from 'idb-keyval'
import type { HistoryEntry } from '../lib/types.ts'

const STORAGE_KEY = 'yt-vanced-history'

interface HistoryState {
  entries: HistoryEntry[]
  initialized: boolean
}

interface HistoryActions {
  init: () => Promise<void>
  add: (entry: Omit<HistoryEntry, 'watchedAt'>) => Promise<void>
  updateProgress: (videoId: string, progressSeconds: number) => Promise<void>
  remove: (videoId: string) => Promise<void>
  clear: () => Promise<void>
}

async function loadEntries(): Promise<HistoryEntry[]> {
  const raw = await get(STORAGE_KEY)
  if (!raw) return []
  return raw as HistoryEntry[]
}

export const useHistoryStore = create<HistoryState & HistoryActions>((set, get) => ({
  entries: [],
  initialized: false,
  init: async () => {
    const entries = await loadEntries()
    set({ entries, initialized: true })
  },
  add: async (entry) => {
    const entries = [
      { ...entry, watchedAt: Date.now() },
      ...get().entries.filter((e) => e.videoId !== entry.videoId),
    ].slice(0, 500)
    await setItem(STORAGE_KEY, entries)
    set({ entries })
  },
  updateProgress: async (videoId, progressSeconds) => {
    const entries = get().entries.map((entry) =>
      entry.videoId === videoId ? { ...entry, progressSeconds } : entry,
    )
    await setItem(STORAGE_KEY, entries)
    set({ entries })
  },
  remove: async (videoId) => {
    const entries = get().entries.filter((entry) => entry.videoId !== videoId)
    await setItem(STORAGE_KEY, entries)
    set({ entries })
  },
  clear: async () => {
    await setItem(STORAGE_KEY, [])
    set({ entries: [] })
  },
}))
