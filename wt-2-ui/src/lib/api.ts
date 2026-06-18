import type {
  ChannelDetail,
  PlaylistDetail,
  SearchOpts,
  SearchResult,
  SponsorSegment,
  StreamMap,
  VideoDetail,
  VideoSummary,
} from './types.ts'
import { MockClient } from './mockClient.ts'

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

export const client: YtClient =
  import.meta.env.VITE_BACKEND === 'real'
    ? new RealClient(import.meta.env.VITE_BACKEND_URL ?? '/api')
    : new MockClient()
