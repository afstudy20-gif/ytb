import { useEffect } from 'react'
import { Play, Trash2, Pause, RotateCcw } from 'lucide-react'
import { useDownloadsStore } from '../stores/downloads.ts'
import { formatFileSize } from '../lib/format.ts'
import { EmptyState } from '../components/EmptyState.tsx'
import { Skeleton } from '../components/Skeleton.tsx'

export function Downloads() {
  const { items, initialized, init, removeDownload, pauseDownload, resumeDownload } = useDownloadsStore()

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

  if (!items.length) {
    return (
      <EmptyState
        icon={Play}
        title="No downloads yet"
        subtitle="Videos you download will appear here for offline viewing."
      />
    )
  }

  return (
    <div className="flex flex-col gap-2 p-4">
      <h1 className="text-2xl font-bold text-text">Downloads</h1>
      {items.map((item) => (
        <div key={item.id} className="rounded-xl bg-surface p-3">
          <div className="flex gap-3">
            <img
              src={item.thumbnailUrl}
              alt=""
              className="h-20 w-36 shrink-0 rounded-lg object-cover"
              loading="lazy"
            />
            <div className="min-w-0 flex-1">
              <h3 className="line-clamp-2 text-sm font-semibold text-text">{item.title}</h3>
              <p className="text-xs text-subtext">{item.authorName}</p>
              <p className="text-xs text-subtext">
                {item.quality} · {item.audioFormat}
              </p>
            </div>
          </div>
          <div className="mt-3">
            <div className="flex items-center justify-between text-xs text-subtext">
              <span>
                {item.state === 'completed'
                  ? 'Completed'
                  : item.state === 'failed'
                    ? 'Failed'
                    : `${Math.round(item.progress)}%`}
              </span>
              <span>
                {formatFileSize(item.bytesDownloaded)} / {formatFileSize(item.bytesTotal)}
              </span>
            </div>
            <div className="mt-1 h-1.5 w-full rounded bg-border">
              <div
                className={`h-full rounded ${item.state === 'failed' ? 'bg-red-500' : 'bg-accent'}`}
                style={{ width: `${item.progress}%` }}
              />
            </div>
          </div>
          <div className="mt-2 flex items-center gap-2">
            {item.state === 'downloading' ? (
              <button
                type="button"
                onClick={() => pauseDownload(item.id)}
                className="flex items-center gap-1 rounded-full bg-surface-hover px-3 py-1.5 text-xs font-semibold text-text"
              >
                <Pause className="h-3.5 w-3.5" /> Pause
              </button>
            ) : item.state === 'paused' || item.state === 'queued' ? (
              <button
                type="button"
                onClick={() => resumeDownload(item.id)}
                className="flex items-center gap-1 rounded-full bg-surface-hover px-3 py-1.5 text-xs font-semibold text-text"
              >
                <Play className="h-3.5 w-3.5" /> Resume
              </button>
            ) : item.state === 'failed' ? (
              <button
                type="button"
                onClick={() => resumeDownload(item.id)}
                className="flex items-center gap-1 rounded-full bg-surface-hover px-3 py-1.5 text-xs font-semibold text-text"
              >
                <RotateCcw className="h-3.5 w-3.5" /> Retry
              </button>
            ) : (
              <button
                type="button"
                className="flex items-center gap-1 rounded-full bg-surface-hover px-3 py-1.5 text-xs font-semibold text-text"
              >
                <Play className="h-3.5 w-3.5" /> Play
              </button>
            )}
            <button
              type="button"
              onClick={() => removeDownload(item.id)}
              className="ml-auto rounded-full p-2 text-subtext hover:bg-surface-hover hover:text-red-500"
              aria-label="Delete download"
            >
              <Trash2 className="h-4 w-4" />
            </button>
          </div>
        </div>
      ))}
    </div>
  )
}
