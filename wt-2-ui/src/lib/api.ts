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
import { isTauriEnv, TauriClient } from './tauriClient.ts'

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

function resolveClient(): YtClient {
  if (import.meta.env.VITE_BACKEND === 'real') {
    return new RealClient(import.meta.env.VITE_BACKEND_URL ?? '/api')
  }
  if (import.meta.env.VITE_TAURI === '1' || isTauriEnv()) {
    return new TauriClient()
  }
  return new MockClient()
}

class LazyClient implements YtClient {
  private _client?: YtClient

  private getClient(): YtClient {
    if (!this._client) {
      this._client = resolveClient()
    }
    return this._client
  }

  search(query: string, opts?: SearchOpts): Promise<SearchResult> {
    return this.getClient().search(query, opts)
  }
  trending(region?: string): Promise<VideoSummary[]> {
    return this.getClient().trending(region)
  }
  video(id: string): Promise<VideoDetail> {
    return this.getClient().video(id)
  }
  streams(id: string): Promise<StreamMap> {
    return this.getClient().streams(id)
  }
  channel(id: string): Promise<ChannelDetail> {
    return this.getClient().channel(id)
  }
  playlist(id: string): Promise<PlaylistDetail> {
    return this.getClient().playlist(id)
  }
  sponsorBlockSegments(id: string, categories: string[]): Promise<SponsorSegment[]> {
    return this.getClient().sponsorBlockSegments(id, categories)
  }
  returnYouTubeDislike(id: string): Promise<{ likes: number; dislikes: number }> {
    return this.getClient().returnYouTubeDislike(id)
  }
}

export const client: YtClient = new LazyClient()
