import type {
  ChannelDetail,
  Format,
  PlaylistDetail,
  SearchItem,
  SearchOpts,
  SearchResult,
  SponsorSegment,
  StreamMap,
  VideoDetail,
  VideoSummary,
} from './types.ts'

export interface YtClient {
  search(query: string, opts?: SearchOpts): Promise<SearchResult>
  trending(region?: string): Promise<VideoSummary[]>
  video(id: string): Promise<VideoDetail>
  streams(id: string): Promise<StreamMap>
  channel(id: string): Promise<ChannelDetail>
  playlist(id: string): Promise<PlaylistDetail>
  sponsorBlockSegments(id: string, categories: string[]): Promise<SponsorSegment[]>
  returnYouTubeDislike(id: string): Promise<{ likes: number; dislikes: number }>
}

export class RealClient implements YtClient {
  private baseUrl: string

  constructor(baseUrl: string) {
    this.baseUrl = baseUrl.replace(/\/$/, '')
  }

  async search(query: string, opts?: SearchOpts): Promise<SearchResult> {
    const params = new URLSearchParams({ q: query })
    if (opts?.continuation) params.set('continuation', opts.continuation)
    if (opts?.filter && opts.filter !== 'all') params.set('filter', opts.filter)
    const res = await fetch(`${this.baseUrl}/search?${params.toString()}`)
    if (!res.ok) throw new Error(`search failed: ${res.status}`)
    return (await res.json()) as SearchResult
  }

  async trending(region = 'US'): Promise<VideoSummary[]> {
    const res = await fetch(`${this.baseUrl}/trending?region=${region}`)
    if (!res.ok) throw new Error(`trending failed: ${res.status}`)
    return (await res.json()) as VideoSummary[]
  }

  async video(id: string): Promise<VideoDetail> {
    const res = await fetch(`${this.baseUrl}/videos/${id}`)
    if (!res.ok) throw new Error(`video failed: ${res.status}`)
    return (await res.json()) as VideoDetail
  }

  async streams(id: string): Promise<StreamMap> {
    const res = await fetch(`${this.baseUrl}/streams/${id}`)
    if (!res.ok) throw new Error(`streams failed: ${res.status}`)
    return (await res.json()) as StreamMap
  }

  async channel(id: string): Promise<ChannelDetail> {
    const res = await fetch(`${this.baseUrl}/channels/${id}`)
    if (!res.ok) throw new Error(`channel failed: ${res.status}`)
    return (await res.json()) as ChannelDetail
  }

  async playlist(id: string): Promise<PlaylistDetail> {
    const res = await fetch(`${this.baseUrl}/playlists/${id}`)
    if (!res.ok) throw new Error(`playlist failed: ${res.status}`)
    return (await res.json()) as PlaylistDetail
  }

  async sponsorBlockSegments(
    id: string,
    categories: string[],
  ): Promise<SponsorSegment[]> {
    const params = new URLSearchParams({ videoId: id })
    categories.forEach((c) => params.append('category', c))
    const res = await fetch(`${this.baseUrl}/sponsorblock?${params.toString()}`)
    if (!res.ok) throw new Error(`sponsorblock failed: ${res.status}`)
    return (await res.json()) as SponsorSegment[]
  }

  async returnYouTubeDislike(id: string): Promise<{ likes: number; dislikes: number }> {
    const res = await fetch(`${this.baseUrl}/ryd/${id}`)
    if (!res.ok) throw new Error(`ryd failed: ${res.status}`)
    return (await res.json()) as { likes: number; dislikes: number }
  }
}

interface RustSearchResultItem {
  id: string
  title: string
  thumbnail: string
  author: string
}

interface RustVideoDetail {
  id: string
  title: string
  description: string
  author: string
  channel_id: string
  view_count: string
  likes: number | null
  duration: number | null
  thumbnail: string
}

interface RustStream {
  itag: number
  url: string
  mime_type: string
  bitrate: number | null
  width: number | null
  height: number | null
  quality_label: string | null
}

interface RustStreamMap {
  progressive: RustStream[]
  adaptive_video: RustStream[]
  adaptive_audio: RustStream[]
  hls_manifest_url: string | null
}

interface RustChannelDetail {
  id: string
  author: string
  subscribers: string | null
  video_count: string | null
  description: string
}

interface RustPlaylistDetail {
  id: string
  title: string
  author: string | null
  video_count: number | null
}

function searchItemToType(item: RustSearchResultItem): SearchItem {
  return {
    type: 'video',
    id: item.id,
    title: item.title,
    author: {
      id: '',
      name: item.author,
      avatarUrl: '',
      verified: false,
    },
    thumbnails: [{ url: item.thumbnail, width: 320, height: 180 }],
    durationSeconds: 0,
    viewCount: 0,
    publishedText: '',
  }
}

function streamsToFormats(map: RustStreamMap): { formats: Format[]; adaptiveFormats: Format[] } {
  return {
    formats: map.progressive.map((s) => ({
      itag: s.itag,
      qualityLabel: s.quality_label ?? `${s.height ?? 0}p`,
      mimeType: s.mime_type,
      bitrate: s.bitrate ?? 0,
      url: s.url,
      audioOnly: false,
    })),
    adaptiveFormats: [
      ...map.adaptive_video.map((s) => ({
        itag: s.itag,
        qualityLabel: s.quality_label ?? `${s.height ?? 0}p`,
        mimeType: s.mime_type,
        bitrate: s.bitrate ?? 0,
        url: s.url,
        audioOnly: false,
      })),
      ...map.adaptive_audio.map((s) => ({
        itag: s.itag,
        qualityLabel: s.quality_label ?? 'Audio only',
        mimeType: s.mime_type,
        bitrate: s.bitrate ?? 0,
        url: s.url,
        audioOnly: true,
      })),
    ],
  }
}

class TauriClient implements YtClient {
  async search(query: string, _opts?: SearchOpts): Promise<SearchResult> {
    const result = await invoke<{ items: RustSearchResultItem[] }>('search', { args: { query } })
    return {
      items: result.items.map(searchItemToType),
      estimatedResults: result.items.length,
    }
  }

  async trending(_region = 'US'): Promise<VideoSummary[]> {
    return []
  }

  async video(id: string): Promise<VideoDetail> {
    const result = await invoke<RustVideoDetail>('video', { args: { id } })
    return {
      id: result.id,
      title: result.title,
      description: result.description,
      viewCount: 0,
      likeCount: result.likes ?? 0,
      publishedText: '',
      durationSeconds: result.duration ?? 0,
      thumbnails: [{ url: result.thumbnail, width: 320, height: 180 }],
      keywords: [],
      chapters: [],
      author: {
        id: result.channel_id,
        name: result.author,
        avatarUrl: '',
        verified: false,
      },
    }
  }

  async streams(id: string): Promise<StreamMap> {
    const result = await invoke<RustStreamMap>('streams', { args: { id } })
    const conv = streamsToFormats(result)
    return {
      videoId: id,
      formats: conv.formats,
      adaptiveFormats: conv.adaptiveFormats,
      expiresInSeconds: 0,
    }
  }

  async channel(id: string): Promise<ChannelDetail> {
    const result = await invoke<RustChannelDetail>('channel', { args: { id } })
    return {
      id: result.id,
      name: result.author,
      avatarUrl: '',
      subscriberCount: 0,
      verified: false,
      description: result.description,
      videoCount: 0,
      videos: [],
      playlists: [],
    }
  }

  async playlist(id: string): Promise<PlaylistDetail> {
    const result = await invoke<RustPlaylistDetail>('playlist', { args: { id } })
    return {
      id: result.id,
      title: result.title,
      description: '',
      videoCount: result.video_count ?? 0,
      thumbnails: [],
      videos: [],
      author: {
        id: '',
        name: result.author ?? '',
        avatarUrl: '',
        verified: false,
      },
    }
  }

  async sponsorBlockSegments(_id: string, _categories: string[]): Promise<SponsorSegment[]> {
    return []
  }

  async returnYouTubeDislike(_id: string): Promise<{ likes: number; dislikes: number }> {
    return { likes: 0, dislikes: 0 }
  }
}

async function invoke<T>(cmd: string, args: Record<string, unknown>): Promise<T> {
  const { invoke: tauriInvoke } = await import('@tauri-apps/api/core')
  const result = await tauriInvoke<T>(cmd, args)
  return result
}

export const client: YtClient = new TauriClient()
