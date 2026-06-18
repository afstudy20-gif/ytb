import { Link } from 'wouter'
import { ListVideo } from 'lucide-react'
import type { PlaylistSummary } from '../lib/types.ts'

interface PlaylistCardProps {
  playlist: PlaylistSummary
}

export function PlaylistCard({ playlist }: PlaylistCardProps) {
  return (
    <Link
      href={`/playlist/${encodeURIComponent(playlist.id)}`}
      className="flex gap-3 rounded-xl p-3 transition-colors hover:bg-surface-hover focus-visible:outline focus-visible:outline-2 focus-visible:outline-accent"
    >
      <div className="relative h-24 w-40 shrink-0 overflow-hidden rounded-lg bg-surface">
        <img
          src={playlist.thumbnailUrl}
          alt=""
          className="h-full w-full object-cover"
          loading="lazy"
        />
        <div className="absolute inset-y-0 right-0 flex w-10 flex-col items-center justify-center bg-black/70 text-white">
          <span className="text-xs font-semibold">{playlist.videoCount}</span>
          <ListVideo className="h-4 w-4" aria-hidden="true" />
        </div>
      </div>
      <div className="min-w-0 flex-1 py-1">
        <h3 className="line-clamp-2 text-sm font-semibold text-text">{playlist.title}</h3>
        <p className="mt-1 text-xs text-subtext">{playlist.author.name}</p>
        <p className="text-xs text-subtext">Playlist · {playlist.videoCount} videos</p>
      </div>
    </Link>
  )
}
