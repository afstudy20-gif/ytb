import { VideoCard } from './VideoCard.tsx'
import type { VideoSummary } from '../lib/types.ts'
import { Skeleton } from './Skeleton.tsx'

interface RelatedVideosProps {
  videos: VideoSummary[] | undefined
  isLoading: boolean
}

export function RelatedVideos({ videos, isLoading }: RelatedVideosProps) {
  return (
    <section className="border-t border-border px-2 py-3">
      <h3 className="px-2 text-base font-bold text-text">Related videos</h3>
      {isLoading ? (
        <div className="flex flex-col gap-1">
          {Array.from({ length: 4 }).map((_, i) => (
            <Skeleton key={i} className="h-28 w-full rounded-lg" />
          ))}
        </div>
      ) : (
        <div className="flex flex-col gap-1">
          {videos?.map((video) => (
            <VideoCard key={video.id} video={video} size="sm" />
          ))}
        </div>
      )}
    </section>
  )
}
