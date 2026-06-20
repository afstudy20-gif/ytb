import { invoke } from '@tauri-apps/api/core'
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
import type { YtClient } from './api.ts'

export class TauriClient implements YtClient {
  async search(query: string, opts?: SearchOpts): Promise<SearchResult> {
    return invoke<SearchResult>('yt_search', {
      request: {
        query,
        continuation: opts?.continuation,
        filter: opts?.filter,
      },
    })
  }

  async trending(region = 'US'): Promise<VideoSummary[]> {
    return invoke<VideoSummary[]>('yt_trending', { region })
  }

  async video(id: string): Promise<VideoDetail> {
    return invoke<VideoDetail>('yt_video', { id })
  }

  async streams(id: string): Promise<StreamMap> {
    return invoke<StreamMap>('yt_streams', { id })
  }

  async channel(id: string): Promise<ChannelDetail> {
    return invoke<ChannelDetail>('yt_channel', { id })
  }

  async playlist(id: string): Promise<PlaylistDetail> {
    return invoke<PlaylistDetail>('yt_playlist', { id })
  }

  async sponsorBlockSegments(
    id: string,
    categories: string[],
  ): Promise<SponsorSegment[]> {
    return invoke<SponsorSegment[]>('yt_sponsor_block', { id, categories })
  }

  async returnYouTubeDislike(
    id: string,
  ): Promise<{ likes: number; dislikes: number }> {
    return invoke<{ likes: number; dislikes: number }>('yt_return_youtube_dislike', {
      id,
    })
  }
}

export function isTauriEnv(): boolean {
  return (
    typeof window !== 'undefined' &&
    // @ts-expect-error Tauri exposes __TAURI__ on window in the app runtime.
    Boolean(window.__TAURI__)
  )
}
