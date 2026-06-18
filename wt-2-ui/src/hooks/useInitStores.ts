import { useEffect } from 'react'
import { useDownloadsStore } from '../stores/downloads.ts'
import { useHistoryStore } from '../stores/history.ts'
import { useLikesStore } from '../stores/likes.ts'
import { usePlaylistsStore } from '../stores/playlists.ts'

export function useInitStores() {
  const initDownloads = useDownloadsStore((state) => state.init)
  const initHistory = useHistoryStore((state) => state.init)
  const initLikes = useLikesStore((state) => state.init)
  const initPlaylists = usePlaylistsStore((state) => state.init)

  useEffect(() => {
    void initDownloads()
    void initHistory()
    void initLikes()
    void initPlaylists()
  }, [initDownloads, initHistory, initLikes, initPlaylists])
}
