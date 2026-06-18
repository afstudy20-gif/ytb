import { useEffect, useState } from 'react'
import { ListVideo, Plus, Trash2 } from 'lucide-react'
import { usePlaylistsStore } from '../stores/playlists.ts'
import { EmptyState } from '../components/EmptyState.tsx'
import { Skeleton } from '../components/Skeleton.tsx'

export function Playlists() {
  const { playlists, initialized, init, create, remove } = usePlaylistsStore()
  const [newName, setNewName] = useState('')

  useEffect(() => {
    void init()
  }, [init])

  const handleCreate = (e: React.FormEvent) => {
    e.preventDefault()
    if (!newName.trim()) return
    void create(newName.trim())
    setNewName('')
  }

  if (!initialized) {
    return (
      <div className="p-4">
        <Skeleton className="mb-3 h-6 w-32" />
        {Array.from({ length: 3 }).map((_, i) => (
          <Skeleton key={i} className="mb-2 h-16 w-full rounded-xl" />
        ))}
      </div>
    )
  }

  return (
    <div className="flex flex-col gap-2 p-4">
      <h1 className="text-2xl font-bold text-text">Playlists</h1>
      <form onSubmit={handleCreate} className="flex gap-2">
        <input
          type="text"
          value={newName}
          onChange={(e) => setNewName(e.target.value)}
          placeholder="New playlist"
          className="flex-1 rounded-lg bg-surface px-3 py-2 text-sm text-text outline-none placeholder:text-subtext"
        />
        <button
          type="submit"
          className="flex items-center gap-1 rounded-lg bg-accent px-3 py-2 text-sm font-semibold text-white"
        >
          <Plus className="h-4 w-4" /> Add
        </button>
      </form>
      {!playlists.length ? (
        <EmptyState icon={ListVideo} title="No playlists" subtitle="Create a playlist to organize your favorite videos." />
      ) : (
        playlists.map((playlist) => (
          <div
            key={playlist.id}
            className="flex items-center justify-between rounded-xl bg-surface p-3"
          >
            <div>
              <h3 className="text-sm font-semibold text-text">{playlist.name}</h3>
              <p className="text-xs text-subtext">{playlist.videoIds.length} videos</p>
            </div>
            <button
              type="button"
              onClick={() => remove(playlist.id)}
              className="rounded-full p-2 text-subtext hover:text-red-500"
              aria-label="Delete playlist"
            >
              <Trash2 className="h-4 w-4" />
            </button>
          </div>
        ))
      )}
    </div>
  )
}
