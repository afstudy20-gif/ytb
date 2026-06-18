import { useQuery } from '@tanstack/react-query'
import { client } from '../lib/api.ts'
import { VideoCard } from '../components/VideoCard.tsx'
import { VideoCardSkeleton } from '../components/Skeleton.tsx'
import { PullToRefresh } from '../components/PullToRefresh.tsx'
import { EmptyState } from '../components/EmptyState.tsx'
import { WifiOff } from 'lucide-react'

export function Home() {
  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ['trending'],
    queryFn: () => client.trending('US'),
  })

  return (
    <PullToRefresh onRefresh={() => void refetch()}>
      <div className="px-2 pt-2">
        <h1 className="sr-only">Home</h1>
        {isLoading ? (
          <div className="grid grid-cols-1 gap-2 sm:grid-cols-2 lg:grid-cols-3">
            {Array.from({ length: 8 }).map((_, i) => (
              <VideoCardSkeleton key={i} />
            ))}
          </div>
        ) : error ? (
          <EmptyState
            icon={WifiOff}
            title="Something went wrong"
            subtitle="Pull down to refresh or check your connection."
          />
        ) : data?.length ? (
          <div className="grid grid-cols-1 gap-2 sm:grid-cols-2 lg:grid-cols-3">
            {data.map((video) => (
              <VideoCard key={video.id} video={video} />
            ))}
          </div>
        ) : (
          <EmptyState
            icon={WifiOff}
            title="No videos found"
            subtitle="Try refreshing the feed."
          />
        )}
      </div>
    </PullToRefresh>
  )
}
