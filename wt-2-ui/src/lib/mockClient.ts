import type {
  Author,
  ChannelDetail,
  ChannelSummary,
  Comment,
  CommentThread,
  PlaylistDetail,
  PlaylistSummary,
  SearchResult,
  SponsorSegment,
  StreamMap,
  VideoDetail,
  VideoSummary,
} from './types.ts'
import type { YtClient } from './api.ts'

const MOCK_THUMBNAILS = [
  'https://images.unsplash.com/photo-1498050108023-c5249f4df085?w=640&q=80',
  'https://images.unsplash.com/photo-1518770660439-4636190af475?w=640&q=80',
  'https://images.unsplash.com/photo-1550745165-9bc0b252726f?w=640&q=80',
  'https://images.unsplash.com/photo-1535223289827-42f1e9919769?w=640&q=80',
  'https://images.unsplash.com/photo-1526374965328-7f61d4dc18c5?w=640&q=80',
  'https://images.unsplash.com/photo-1504639725590-34d0984388bd?w=640&q=80',
  'https://images.unsplash.com/photo-1587620962725-abab7fe55159?w=640&q=80',
  'https://images.unsplash.com/photo-1555066931-4365d14bab8c?w=640&q=80',
]

const MOCK_AVATARS = [
  'https://images.unsplash.com/photo-1535713875002-d1d0cf377fde?w=120&q=80',
  'https://images.unsplash.com/photo-1494790108377-be9c29b29330?w=120&q=80',
  'https://images.unsplash.com/photo-1527980965255-d3b416303d12?w=120&q=80',
  'https://images.unsplash.com/photo-1438761681033-6461ffad8d80?w=120&q=80',
  'https://images.unsplash.com/photo-1472099645785-5658abf4ff4e?w=120&q=80',
]

const MOCK_CHANNELS = [
  'Tech Deep Dive',
  'Maker Studios',
  'City Walks',
  'Cooking Simplified',
  'Retro Gaming',
  'Space Today',
]

const MOCK_TITLES = [
  'Building a custom mechanical keyboard from scratch',
  'Why Rust is taking over systems programming',
  'A walk through Tokyo at night',
  '10-minute pasta that tastes restaurant quality',
  'The complete history of the Game Boy',
  'James Webb telescope: first images explained',
  'I replaced my entire stack with SQLite',
  'Minimalist desk setup tour 2025',
  'How neural networks actually work',
  'Solo camping in the Scottish Highlands',
]

function hashString(str: string): number {
  let h = 0
  for (let i = 0; i < str.length; i++) {
    h = (h << 5) - h + str.charCodeAt(i)
    h |= 0
  }
  return Math.abs(h)
}

function seededChoice<T>(seed: number, arr: readonly T[]): T {
  return arr[seed % arr.length]
}

function makeAuthor(seed: number): Author {
  return {
    id: `channel-${seed}`,
    name: seededChoice(seed, MOCK_CHANNELS),
    avatarUrl: seededChoice(seed, MOCK_AVATARS),
    subscriberCount: 100_000 + (seed % 9_900_000),
    verified: seed % 3 === 0,
  }
}

export function makeVideoSummary(seed: number, overrides?: Partial<VideoSummary>): VideoSummary {
  const base = seed % MOCK_TITLES.length
  const duration = 120 + (seed % 5400)
  return {
    type: 'video',
    id: `video-${seed}`,
    title: overrides?.title ?? MOCK_TITLES[base],
    author: overrides?.author ?? makeAuthor(seed),
    thumbnails: [
      {
        url: seededChoice(seed, MOCK_THUMBNAILS),
        width: 640,
        height: 360,
      },
    ],
    durationSeconds: overrides?.durationSeconds ?? duration,
    viewCount: 50_000 + (seed % 9_950_000),
    publishedText: overrides?.publishedText ?? `${(seed % 11) + 1} days ago`,
  }
}

function makeChannelSummary(seed: number): ChannelSummary {
  return {
    type: 'channel',
    id: `channel-${seed}`,
    name: seededChoice(seed, MOCK_CHANNELS),
    avatarUrl: seededChoice(seed, MOCK_AVATARS),
    subscriberCount: 10_000 + (seed % 9_990_000),
    videoCount: 10 + (seed % 2000),
    verified: seed % 3 === 0,
    descriptionShort: 'Creator of tutorials, reviews, and deep dives.',
  }
}

function makePlaylistSummary(seed: number): PlaylistSummary {
  return {
    type: 'playlist',
    id: `playlist-${seed}`,
    title: `Playlist: ${seededChoice(seed, MOCK_TITLES)}`,
    author: makeAuthor(seed),
    thumbnailUrl: seededChoice(seed, MOCK_THUMBNAILS),
    videoCount: 5 + (seed % 45),
  }
}

function makeComment(seed: number): Comment {
  return {
    id: `comment-${seed}`,
    author: makeAuthor(seed),
    text: seed % 2 === 0
      ? 'This is exactly what I was looking for, thanks for making this!'
      : 'Great breakdown. Would love a follow-up video on this topic.',
    likeCount: seed % 5000,
    publishedText: `${(seed % 11) + 1} days ago`,
    replyCount: seed % 20,
  }
}

function makeComments(seed: number): CommentThread[] {
  return Array.from({ length: 6 }, (_, i) => ({
    comment: makeComment(seed + i),
    replies: i % 2 === 0 ? [makeComment(seed + i + 1000)] : [],
  }))
}

export class MockClient implements YtClient {
  private delay(ms = 400): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms))
  }

  async search(query: string): Promise<SearchResult> {
    await this.delay(500)
    const seed = hashString(query.toLowerCase())
    const items: SearchResult['items'] = []
    for (let i = 0; i < 12; i++) {
      const typeSeed = (seed + i) % 10
      if (typeSeed < 6) items.push(makeVideoSummary(seed + i))
      else if (typeSeed < 8) items.push(makeChannelSummary(seed + i))
      else items.push(makePlaylistSummary(seed + i))
    }
    return {
      items,
      estimatedResults: 100_000 + (seed % 900_000),
    }
  }

  async trending(region = 'US'): Promise<VideoSummary[]> {
    await this.delay(600)
    const seed = hashString(region)
    return Array.from({ length: 16 }, (_, i) => makeVideoSummary(seed + i))
  }

  async video(id: string): Promise<VideoDetail> {
    await this.delay(400)
    const seed = hashString(id)
    const summary = makeVideoSummary(seed)
    return {
      ...summary,
      description:
        'This is a mock video description used for demo purposes.\n\n' +
        'It supports multiple lines and would normally contain chapters, links, and credits.',
      likeCount: 2000 + (seed % 498_000),
      keywords: ['demo', 'mock', 'youtube', 'vanced'],
      chapters:
        seed % 2 === 0
          ? [
              { title: 'Introduction', startSeconds: 0, thumbnailUrl: summary.thumbnails[0].url },
              { title: 'Main topic', startSeconds: 120, thumbnailUrl: summary.thumbnails[0].url },
              { title: 'Deep dive', startSeconds: 300, thumbnailUrl: summary.thumbnails[0].url },
              { title: 'Conclusion', startSeconds: 480, thumbnailUrl: summary.thumbnails[0].url },
            ]
          : [],
    }
  }

  async streams(id: string): Promise<StreamMap> {
    await this.delay(300)
    const seed = hashString(id)
    // These sample videos have permissive CORS headers so they work inside
    // mobile WebViews and cross-origin iframes without ORB blocking.
    const samples = [
      'https://interactive-examples.mdn.mozilla.net/media/cc0-videos/flower.mp4',
      'https://media.w3.org/2010/05/sintel/trailer.mp4',
      'https://sample-videos.com/video321/mp4/720/big_buck_bunny_720p_1mb.mp4',
    ]
    const url = seededChoice(seed, samples)
    return {
      videoId: id,
      formats: [
        { itag: 18, qualityLabel: '360p', mimeType: 'video/mp4', bitrate: 500_000, url, audioOnly: false },
        { itag: 22, qualityLabel: '720p', mimeType: 'video/mp4', bitrate: 2_000_000, url, audioOnly: false },
      ],
      adaptiveFormats: [
        { itag: 137, qualityLabel: '1080p', mimeType: 'video/mp4', bitrate: 4_000_000, url, audioOnly: false },
        { itag: 140, qualityLabel: 'audio', mimeType: 'audio/mp4', bitrate: 128_000, url, audioOnly: true },
      ],
      expiresInSeconds: 600,
    }
  }

  async channel(id: string): Promise<ChannelDetail> {
    await this.delay(500)
    const seed = hashString(id)
    const summary = makeChannelSummary(seed)
    return {
      ...summary,
      bannerUrl: seededChoice(seed, MOCK_THUMBNAILS),
      description: 'A channel full of useful and entertaining content.',
      videos: Array.from({ length: 12 }, (_, i) => makeVideoSummary(seed + i)),
      playlists: Array.from({ length: 4 }, (_, i) => makePlaylistSummary(seed + i)),
    }
  }

  async playlist(id: string): Promise<PlaylistDetail> {
    await this.delay(500)
    const seed = hashString(id)
    const summary = makePlaylistSummary(seed)
    return {
      ...summary,
      description: 'A curated playlist for your viewing pleasure.',
      thumbnails: [
        { url: summary.thumbnailUrl, width: 640, height: 360 },
      ],
      videos: Array.from({ length: summary.videoCount }, (_, i) => makeVideoSummary(seed + i)),
    }
  }

  async sponsorBlockSegments(id: string, categories: string[]): Promise<SponsorSegment[]> {
    await this.delay(200)
    const seed = hashString(id)
    if (seed % 3 === 0 || categories.length === 0) return []
    return [
      { category: 'sponsor', segment: [15, 35] as [number, number], UUID: `seg-${id}-1` },
      { category: 'intro', segment: [120, 132] as [number, number], UUID: `seg-${id}-2` },
    ].filter((s) => categories.includes(s.category))
  }

  async returnYouTubeDislike(id: string): Promise<{ likes: number; dislikes: number }> {
    await this.delay(150)
    const seed = hashString(id)
    const likes = 2000 + (seed % 498_000)
    const dislikes = Math.floor(likes / (5 + (seed % 15)))
    return { likes, dislikes }
  }

  getMockComments(videoId: string): CommentThread[] {
    return makeComments(hashString(videoId))
  }
}
