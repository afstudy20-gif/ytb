import { useEffect } from 'react'
import { ThumbsUp } from 'lucide-react'
import { Link } from 'wouter'
import { useLikesStore } from '../stores/likes.ts'
import { EmptyState } from '../components/EmptyState.tsx'
import { Skeleton } from '../components/Skeleton.tsx'

export function Liked() {
  const { videos, initialized, init } = useLikesStore()

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

  if (!videos.length) {
    return <EmptyState icon={ThumbsUp} title="No liked videos" subtitle="Tap the like button on videos you enjoy." />
  }

  return (
    <div className="flex flex-col gap-2 p-4">
      <h1 className="text-2xl font-bold text-text">Liked videos</h1>
      {videos.map((video) => (
        <Link
          key={video.videoId}
          href={`/watch/${encodeURIComponent(video.videoId)}`}
          className="flex gap-3 rounded-xl bg-surface p-3 transition-colors hover:bg-surface-hover"
        >
          <img
            src={video.thumbnailUrl}
            alt=""
            className="h-20 w-36 shrink-0 rounded-lg object-cover"
            loading="lazy"
          />
          <div className="min-w-0 flex-1">
            <h3 className="line-clamp-2 text-sm font-semibold text-text">{video.title}</h3>
            <p className="text-xs text-subtext">{video.authorName}</p>
          </div>
        </Link>
      ))}
    </div>
  )
}
