import { create } from 'zustand'
import { get, set as setItem } from 'idb-keyval'
import type { Playlist } from '../lib/types.ts'

const STORAGE_KEY = 'yt-vanced-playlists'

interface PlaylistsState {
  playlists: Playlist[]
  initialized: boolean
}

interface PlaylistsActions {
  init: () => Promise<void>
  create: (name: string) => Promise<void>
  rename: (id: string, name: string) => Promise<void>
  remove: (id: string) => Promise<void>
  addVideo: (id: string, videoId: string) => Promise<void>
  removeVideo: (id: string, videoId: string) => Promise<void>
}

async function loadPlaylists(): Promise<Playlist[]> {
  const raw = await get(STORAGE_KEY)
  if (!raw) return []
  return raw as Playlist[]
}

export const usePlaylistsStore = create<PlaylistsState & PlaylistsActions>((set, get) => ({
  playlists: [],
  initialized: false,
  init: async () => {
    const playlists = await loadPlaylists()
    set({ playlists, initialized: true })
  },
  create: async (name) => {
    const now = Date.now()
    const playlist: Playlist = {
      id: `pl-${now}`,
      name,
      videoIds: [],
      createdAt: now,
      updatedAt: now,
    }
    const playlists = [playlist, ...get().playlists]
    await setItem(STORAGE_KEY, playlists)
    set({ playlists })
  },
  rename: async (id, name) => {
    const now = Date.now()
    const playlists = get().playlists.map((pl) =>
      pl.id === id ? { ...pl, name, updatedAt: now } : pl,
    )
    await setItem(STORAGE_KEY, playlists)
    set({ playlists })
  },
  remove: async (id) => {
    const playlists = get().playlists.filter((pl) => pl.id !== id)
    await setItem(STORAGE_KEY, playlists)
    set({ playlists })
  },
  addVideo: async (id, videoId) => {
    const now = Date.now()
    const playlists = get().playlists.map((pl) =>
      pl.id === id && !pl.videoIds.includes(videoId)
        ? { ...pl, videoIds: [...pl.videoIds, videoId], updatedAt: now }
        : pl,
    )
    await setItem(STORAGE_KEY, playlists)
    set({ playlists })
  },
  removeVideo: async (id, videoId) => {
    const now = Date.now()
    const playlists = get().playlists.map((pl) =>
      pl.id === id
        ? { ...pl, videoIds: pl.videoIds.filter((vid) => vid !== videoId), updatedAt: now }
        : pl,
    )
    await setItem(STORAGE_KEY, playlists)
    set({ playlists })
  },
}))
