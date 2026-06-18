export interface Thumbnail {
  url: string
  width: number
  height: number
}

export interface Author {
  id: string
  name: string
  avatarUrl: string
  subscriberCount?: number
  verified: boolean
}

export interface VideoSummary {
  type: 'video'
  id: string
  title: string
  author: Author
  thumbnails: Thumbnail[]
  durationSeconds: number
  viewCount: number
  publishedText: string
}

export interface ChannelSummary {
  type: 'channel'
  id: string
  name: string
  avatarUrl: string
  subscriberCount: number
  videoCount: number
  verified: boolean
  descriptionShort: string
}

export interface PlaylistSummary {
  type: 'playlist'
  id: string
  title: string
  author: Author
  thumbnailUrl: string
  videoCount: number
}

export type SearchItem = VideoSummary | ChannelSummary | PlaylistSummary

export interface SearchOpts {
  continuation?: string
  filter?: 'videos' | 'channels' | 'playlists' | 'all'
}

export interface SearchResult {
  items: SearchItem[]
  continuation?: string
  estimatedResults: number
}

export interface Chapter {
  title: string
  startSeconds: number
  thumbnailUrl: string
}

export interface VideoDetail {
  id: string
  title: string
  author: Author
  description: string
  viewCount: number
  likeCount: number
  publishedText: string
  durationSeconds: number
  thumbnails: Thumbnail[]
  keywords: string[]
  chapters: Chapter[]
}

export interface Format {
  itag: number
  qualityLabel: string
  mimeType: string
  bitrate: number
  url: string
  audioOnly: boolean
}

export interface StreamMap {
  videoId: string
  formats: Format[]
  adaptiveFormats: Format[]
  expiresInSeconds: number
}

export interface ChannelDetail {
  id: string
  name: string
  avatarUrl: string
  bannerUrl?: string
  subscriberCount: number
  verified: boolean
  description: string
  videoCount: number
  videos: VideoSummary[]
  playlists: PlaylistSummary[]
}

export interface PlaylistDetail {
  id: string
  title: string
  author: Author
  description: string
  videoCount: number
  thumbnails: Thumbnail[]
  videos: VideoSummary[]
}

export interface SponsorSegment {
  category: string
  segment: [number, number]
  UUID: string
}

export interface Comment {
  id: string
  author: Author
  text: string
  likeCount: number
  publishedText: string
  replyCount: number
}

export interface CommentThread {
  comment: Comment
  replies: Comment[]
}

export type DownloadQuality = 'audio-only' | '360p' | '720p' | '1080p'
export type AudioFormat = 'm4a' | 'opus'
export type ThemeMode = 'system' | 'dark' | 'light'

export interface AppSettings {
  sponsorBlockCategories: string[]
  returnYouTubeDislike: boolean
  defaultDownloadQuality: DownloadQuality
  defaultAudioFormat: AudioFormat
  wifiOnlyDownloads: boolean
  theme: ThemeMode
}

export interface DownloadItem {
  id: string
  videoId: string
  title: string
  authorName: string
  thumbnailUrl: string
  quality: DownloadQuality
  audioFormat: AudioFormat
  state: 'queued' | 'downloading' | 'paused' | 'completed' | 'failed'
  progress: number
  bytesTotal: number
  bytesDownloaded: number
  error?: string
  createdAt: number
  completedAt?: number
}

export interface HistoryEntry {
  videoId: string
  title: string
  authorName: string
  thumbnailUrl: string
  watchedAt: number
  progressSeconds: number
  durationSeconds: number
}

export interface LikedVideo {
  videoId: string
  title: string
  authorName: string
  thumbnailUrl: string
  likedAt: number
}

export interface Playlist {
  id: string
  name: string
  videoIds: string[]
  createdAt: number
  updatedAt: number
}
