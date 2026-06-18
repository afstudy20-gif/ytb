import { useEffect } from 'react'
import { History as HistoryIcon, Trash2 } from 'lucide-react'
import { Link } from 'wouter'
import { useHistoryStore } from '../stores/history.ts'
import { formatDuration } from '../lib/format.ts'
import { EmptyState } from '../components/EmptyState.tsx'
import { Skeleton } from '../components/Skeleton.tsx'

export function History() {
  const { entries, initialized, init, remove, clear } = useHistoryStore()

  useEffect(() => {
    void init()
  }, [init])

  if (!initialized) {
    return (
      <div className="p-4">
        <Skeleton className="mb-3 h-6 w-32" />
        {Array.from({ length: 4 }).map((_, i) => (
          <Skeleton key={i} className="mb-2 h-20 w-full rounded-xl" />
        ))}
      </div>
    )
  }

  if (!entries.length) {
    return <EmptyState icon={HistoryIcon} title="No watch history" subtitle="Videos you watch will appear here." />
  }

  return (
    <div className="flex flex-col gap-2 p-4">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold text-text">History</h1>
        <button
          type="button"
          onClick={() => void clear()}
          className="text-sm font-semibold text-red-500"
        >
          Clear all
        </button>
      </div>
      {entries.map((entry) => (
        <Link
          key={entry.videoId}
          href={`/watch/${encodeURIComponent(entry.videoId)}`}
          className="flex gap-3 rounded-xl bg-surface p-3 transition-colors hover:bg-surface-hover"
        >
          <img
            src={entry.thumbnailUrl}
            alt=""
            className="h-20 w-36 shrink-0 rounded-lg object-cover"
            loading="lazy"
          />
          <div className="min-w-0 flex-1">
            <h3 className="line-clamp-2 text-sm font-semibold text-text">{entry.title}</h3>
            <p className="text-xs text-subtext">{entry.authorName}</p>
            <p className="text-xs text-subtext">
              {formatDuration(entry.progressSeconds)} / {formatDuration(entry.durationSeconds)}
            </p>
          </div>
          <button
            type="button"
            onClick={(e) => {
              e.preventDefault()
              void remove(entry.videoId)
            }}
            className="self-center rounded-full p-2 text-subtext hover:text-red-500"
            aria-label="Remove from history"
          >
            <Trash2 className="h-4 w-4" />
          </button>
        </Link>
      ))}
    </div>
  )
}
