import { Link } from 'wouter'
import { Play, Pause, X, Maximize2, SkipForward } from 'lucide-react'
import { usePlayerStore } from '../stores/player.ts'
import { formatDuration } from '../lib/format.ts'

export function MiniPlayer() {
  const {
    queue,
    currentIndex,
    isPlaying,
    isExpanded,
    currentTime,
    togglePlay,
    expand,
    close,
    next,
  } = usePlayerStore()

  const current = queue[currentIndex]
  if (!current) return null

  const thumbnail = current.thumbnails[0]?.url
  const title = current.title
  const subtitle = current.author.name
  const duration = current.durationSeconds

  if (isExpanded) return null

  return (
    <div className="fixed bottom-14 left-0 right-0 z-50 border-t border-border bg-surface px-3 py-2 shadow-lg">
      <div className="flex items-center gap-3">
        <Link
          href={`/watch/${encodeURIComponent(current.id)}`}
          onClick={() => expand()}
          className="flex min-w-0 flex-1 items-center gap-3"
        >
          <div className="relative h-12 w-20 shrink-0 overflow-hidden rounded bg-bg">
            {thumbnail ? (
              <img src={thumbnail} alt="" className="h-full w-full object-cover" />
            ) : null}
          </div>
          <div className="min-w-0 flex-1">
            <p className="truncate text-sm font-semibold text-text">{title}</p>
            <p className="truncate text-xs text-subtext">{subtitle}</p>
            <div className="mt-1 h-1 w-full rounded bg-border">
              <div
                className="h-full rounded bg-accent"
                style={{ width: `${duration > 0 ? (currentTime / duration) * 100 : 0}%` }}
              />
            </div>
            <p className="text-[10px] text-subtext">
              {formatDuration(currentTime)} / {formatDuration(duration)}
            </p>
          </div>
        </Link>
        <div className="flex items-center gap-1">
          <button
            type="button"
            onClick={() => togglePlay()}
            className="rounded-full p-2 text-text hover:bg-surface-hover"
            aria-label={isPlaying ? 'Pause' : 'Play'}
          >
            {isPlaying ? <Pause className="h-5 w-5" /> : <Play className="h-5 w-5" />}
          </button>
          <button
            type="button"
            onClick={() => next()}
            className="rounded-full p-2 text-text hover:bg-surface-hover"
            aria-label="Next"
          >
            <SkipForward className="h-5 w-5" />
          </button>
          <button
            type="button"
            onClick={() => expand()}
            className="rounded-full p-2 text-text hover:bg-surface-hover"
            aria-label="Expand player"
          >
            <Maximize2 className="h-5 w-5" />
          </button>
          <button
            type="button"
            onClick={() => close()}
            className="rounded-full p-2 text-subtext hover:bg-surface-hover"
            aria-label="Close player"
          >
            <X className="h-5 w-5" />
          </button>
        </div>
      </div>
    </div>
  )
}
