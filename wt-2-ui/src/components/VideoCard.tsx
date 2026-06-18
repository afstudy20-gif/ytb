import { Link } from 'wouter'
import { BadgeCheck } from 'lucide-react'
import type { VideoSummary } from '../lib/types.ts'
import { formatCount, formatDuration } from '../lib/format.ts'
import { Avatar } from './Avatar.tsx'

interface VideoCardProps {
  video: VideoSummary
  size?: 'sm' | 'md'
}

export function VideoCard({ video, size = 'md' }: VideoCardProps) {
  const thumbnail = video.thumbnails[0]?.url
  return (
    <Link
      href={`/watch/${encodeURIComponent(video.id)}`}
      className="group flex flex-col gap-2 overflow-hidden rounded-xl p-2 transition-colors hover:bg-surface-hover focus-visible:outline focus-visible:outline-2 focus-visible:outline-accent"
    >
      <div className="relative aspect-video w-full overflow-hidden rounded-lg bg-surface">
        {thumbnail ? (
          <img
            src={thumbnail}
            alt=""
            className="h-full w-full object-cover transition-transform group-hover:scale-105"
            loading="lazy"
          />
        ) : null}
        <span className="absolute bottom-1.5 right-1.5 rounded bg-black/80 px-1 text-xs font-medium text-white">
          {formatDuration(video.durationSeconds)}
        </span>
      </div>
      <div className="flex gap-3">
        {size === 'md' ? (
          <Avatar src={video.author.avatarUrl} alt={video.author.name} size="md" />
        ) : null}
        <div className="min-w-0 flex-1">
          <h3 className="line-clamp-2 text-sm font-semibold leading-snug text-text">
            {video.title}
          </h3>
          <div className="mt-0.5 flex items-center gap-1 text-xs text-subtext">
            <span className="truncate">{video.author.name}</span>
            {video.author.verified ? (
              <BadgeCheck className="h-3 w-3 shrink-0 text-subtext" aria-label="Verified" />
            ) : null}
          </div>
          <p className="text-xs text-subtext">
            {formatCount(video.viewCount)} views · {video.publishedText}
          </p>
        </div>
      </div>
    </Link>
  )
}
