import { useEffect, useState } from 'react'
import { useSearch } from 'wouter'
import { useQuery } from '@tanstack/react-query'
import { Search as SearchIcon, X } from 'lucide-react'
import { client } from '../lib/api.ts'
import { VideoCard } from '../components/VideoCard.tsx'
import { ChannelCard } from '../components/ChannelCard.tsx'
import { PlaylistCard } from '../components/PlaylistCard.tsx'
import { SearchResultSkeleton } from '../components/Skeleton.tsx'
import { EmptyState } from '../components/EmptyState.tsx'

export function Search() {
  const params = new URLSearchParams(useSearch())
  const initialQuery = params.get('q') ?? ''
  const [query, setQuery] = useState(initialQuery)
  const [submittedQuery, setSubmittedQuery] = useState(initialQuery)

  useEffect(() => {
    setQuery(initialQuery)
    setSubmittedQuery(initialQuery)
  }, [initialQuery])

  const { data, isLoading } = useQuery({
    queryKey: ['search', submittedQuery],
    queryFn: () => client.search(submittedQuery),
    enabled: submittedQuery.length > 0,
  })

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    const trimmed = query.trim()
    if (!trimmed) return
    setSubmittedQuery(trimmed)
    const url = new URL(window.location.href)
    url.searchParams.set('q', trimmed)
    window.history.replaceState({}, '', url.toString())
  }

  return (
    <div className="flex flex-col">
      <form onSubmit={handleSubmit} className="sticky top-0 z-20 border-b border-border bg-bg p-3">
        <div className="flex items-center gap-2 rounded-full bg-surface px-3 py-2">
          <SearchIcon className="h-4 w-4 text-subtext" aria-hidden="true" />
          <input
            type="search"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search videos, channels..."
            className="flex-1 bg-transparent text-sm text-text outline-none placeholder:text-subtext"
            aria-label="Search"
          />
          {query ? (
            <button
              type="button"
              onClick={() => setQuery('')}
              className="rounded-full p-1 text-subtext hover:bg-surface-hover"
              aria-label="Clear search"
            >
              <X className="h-4 w-4" aria-hidden="true" />
            </button>
          ) : null}
        </div>
      </form>

      <div className="px-1 py-2">
        {!submittedQuery ? (
          <EmptyState
            icon={SearchIcon}
            title="Start searching"
            subtitle="Type a query above to find videos, channels, and playlists."
          />
        ) : isLoading ? (
          <div className="flex flex-col gap-1">
            {Array.from({ length: 6 }).map((_, i) => (
              <SearchResultSkeleton key={i} />
            ))}
          </div>
        ) : data?.items.length ? (
          <div className="flex flex-col gap-1">
            {data.items.map((item) => {
              if (item.type === 'video') return <VideoCard key={item.id} video={item} size="sm" />
              if (item.type === 'channel') return <ChannelCard key={item.id} channel={item} />
              return <PlaylistCard key={item.id} playlist={item} />
            })}
          </div>
        ) : (
          <EmptyState
            icon={SearchIcon}
            title="No results"
            subtitle={`We couldn't find anything for "${submittedQuery}".`}
          />
        )}
      </div>
    </div>
  )
}
