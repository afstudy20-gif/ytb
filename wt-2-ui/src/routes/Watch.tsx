import { useParams } from 'wouter'
import { useQuery } from '@tanstack/react-query'
import { useEffect, useState } from 'react'
import {
  ThumbsUp,
  ThumbsDown,
  Share2,
  Download,
  BadgeCheck,
  ChevronDown,
  ChevronUp,
} from 'lucide-react'
import { client } from '../lib/api.ts'
import { usePlayerStore } from '../stores/player.ts'
import { useHistoryStore } from '../stores/history.ts'
import { useLikesStore } from '../stores/likes.ts'
import { useSettingsStore } from '../stores/settings.ts'
import { VideoPlayer } from '../components/VideoPlayer.tsx'
import { CommentsList } from '../components/CommentsList.tsx'
import { RelatedVideos } from '../components/RelatedVideos.tsx'
import { Avatar } from '../components/Avatar.tsx'
import { Skeleton } from '../components/Skeleton.tsx'
import { formatCount } from '../lib/format.ts'
import { makeVideoSummary } from '../lib/mockClient.ts'
import type { CommentThread } from '../lib/types.ts'

export function Watch() {
  const { videoId = '' } = useParams()
  const [descExpanded, setDescExpanded] = useState(false)
  const playVideo = usePlayerStore((state) => state.playVideo)
  const addHistory = useHistoryStore((state) => state.add)
  const toggleLike = useLikesStore((state) => state.toggle)
  const isLiked = useLikesStore((state) => state.isLiked(videoId))
  const rydEnabled = useSettingsStore((state) => state.returnYouTubeDislike)

  const { data: video, isLoading: videoLoading } = useQuery({
    queryKey: ['video', videoId],
    queryFn: () => client.video(videoId),
  })

  const { data: streams, isLoading: streamsLoading } = useQuery({
    queryKey: ['streams', videoId],
    queryFn: () => client.streams(videoId),
  })

  const { data: ryd } = useQuery({
    queryKey: ['ryd', videoId],
    queryFn: () => client.returnYouTubeDislike(videoId),
    enabled: rydEnabled,
  })

  const { data: segments } = useQuery({
    queryKey: ['sponsorblock', videoId],
    queryFn: () => client.sponsorBlockSegments(videoId, ['sponsor', 'intro', 'outro', 'selfpromo']),
  })

  const { data: related } = useQuery({
    queryKey: ['trending-related'],
    queryFn: () => client.trending('US'),
  })

  const [comments] = useState<CommentThread[]>(() => {
    // In a real app this would come from the client; mock data is local.
    return []
  })

  useEffect(() => {
    if (video) {
      playVideo(video)
      void addHistory({
        videoId: video.id,
        title: video.title,
        authorName: video.author.name,
        thumbnailUrl: video.thumbnails[0]?.url ?? '',
        progressSeconds: 0,
        durationSeconds: video.durationSeconds,
      })
    }
  }, [video, playVideo, addHistory])

  if (videoLoading || streamsLoading || !video || !streams) {
    return (
      <div className="flex flex-col">
        <Skeleton className="aspect-video w-full" />
        <div className="p-4">
          <Skeleton className="h-5 w-full" />
          <Skeleton className="mt-2 h-4 w-2/3" />
          <div className="mt-4 flex gap-2">
            <Skeleton className="h-10 w-20 rounded-full" />
            <Skeleton className="h-10 w-20 rounded-full" />
            <Skeleton className="h-10 w-20 rounded-full" />
          </div>
        </div>
      </div>
    )
  }

  const relatedVideos = related?.filter((v) => v.id !== videoId).slice(0, 8) ?? Array.from({ length: 8 }, (_, i) => makeVideoSummary(i + 100))

  return (
    <div className="flex flex-col">
      <VideoPlayer video={video} streams={streams} segments={segments ?? []} />

      <div className="px-4 pt-3">
        <h1 className="text-lg font-bold leading-snug text-text">{video.title}</h1>
        <div className="mt-1 flex items-center gap-2 text-sm text-subtext">
          <span>{formatCount(video.viewCount)} views</span>
          <span>·</span>
          <span>{video.publishedText}</span>
        </div>
      </div>

      <div className="flex items-center justify-between gap-2 px-4 py-3">
        <div className="flex items-center gap-3 overflow-hidden">
          <Avatar src={video.author.avatarUrl} alt={video.author.name} size="md" />
          <div className="min-w-0">
            <div className="flex items-center gap-1">
              <span className="truncate text-sm font-semibold text-text">{video.author.name}</span>
              {video.author.verified ? (
                <BadgeCheck className="h-3.5 w-3.5 shrink-0 text-subtext" aria-label="Verified" />
              ) : null}
            </div>
            <p className="text-xs text-subtext">
              {video.author.subscriberCount ? formatCount(video.author.subscriberCount) : '0'} subscribers
            </p>
          </div>
        </div>
        <button
          type="button"
          className="shrink-0 rounded-full bg-accent px-4 py-2 text-sm font-semibold text-white"
        >
          Subscribe
        </button>
      </div>

      <div className="flex gap-2 overflow-x-auto px-4 pb-3 no-scrollbar">
        <button
          type="button"
          onClick={() =>
            void toggleLike({
              videoId: video.id,
              title: video.title,
              authorName: video.author.name,
              thumbnailUrl: video.thumbnails[0]?.url ?? '',
            })
          }
          className={`flex shrink-0 items-center gap-2 rounded-full bg-surface px-4 py-2 text-sm font-semibold ${
            isLiked ? 'text-accent' : 'text-text'
          }`}
          aria-pressed={isLiked}
        >
          <ThumbsUp className="h-4 w-4" />
          {formatCount(ryd?.likes ?? video.likeCount)}
        </button>
        <button
          type="button"
          className="flex shrink-0 items-center gap-2 rounded-full bg-surface px-4 py-2 text-sm font-semibold text-text"
        >
          <ThumbsDown className="h-4 w-4" />
          {rydEnabled && ryd ? formatCount(ryd.dislikes) : 'Dislike'}
        </button>
        <button
          type="button"
          className="flex shrink-0 items-center gap-2 rounded-full bg-surface px-4 py-2 text-sm font-semibold text-text"
        >
          <Share2 className="h-4 w-4" />
          Share
        </button>
        <button
          type="button"
          className="flex shrink-0 items-center gap-2 rounded-full bg-surface px-4 py-2 text-sm font-semibold text-text"
        >
          <Download className="h-4 w-4" />
          Download
        </button>
      </div>

      <button
        type="button"
        onClick={() => setDescExpanded((e) => !e)}
        className="mx-4 rounded-xl bg-surface p-3 text-left"
        aria-expanded={descExpanded}
      >
        <p className={`text-sm text-text ${descExpanded ? '' : 'line-clamp-2'}`}>{video.description}</p>
        <div className="mt-1 flex items-center gap-1 text-xs text-subtext">
          {descExpanded ? <ChevronUp className="h-4 w-4" /> : <ChevronDown className="h-4 w-4" />}
          <span>{descExpanded ? 'Show less' : 'Show more'}</span>
        </div>
      </button>

      <CommentsList threads={comments} isLoading={false} totalCount={128} />
      <RelatedVideos videos={relatedVideos} isLoading={false} />
    </div>
  )
}
