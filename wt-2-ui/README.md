# VancedTube UI

A polished, mobile-first React UI for a personal YouTube client. Built as a standalone Vite app so it can later be dropped into a Tauri v2 shell with minimal changes.

## Stack

- pnpm + Vite 5 + React 18 + TypeScript 5 (strict)
- TailwindCSS 3.4
- Wouter (routing)
- Zustand (state)
- @tanstack/react-query (data fetching)
- lucide-react (icons)
- idb-keyval (IndexedDB persistence)

## Getting started

```bash
pnpm install
pnpm dev
```

## Verification

```bash
pnpm typecheck   # tsc --noEmit
pnpm lint        # eslint
pnpm build       # production build
```

All three pass in the current tree.

## Routes

| Route | Screen |
|-------|--------|
| `/` | Home feed — trending/recommended video grid with pull-to-refresh |
| `/search?q=...` | Search — mixed results (videos, channels, playlists) |
| `/watch/:videoId` | Watch — player, channel row, description, comments stub, related videos |
| `/library` | Library hub — Downloads, History, Liked, Playlists |
| `/library/downloads` | Downloads list with progress / pause / delete |
| `/library/history` | Watch history |
| `/library/liked` | Liked videos |
| `/library/playlists` | Playlists (create / delete) |
| `/settings` | SponsorBlock, RYD, downloads, theme |

## Backend: mock ↔ real

The backend is hidden behind `src/lib/api.ts`:

```ts
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
```

- `src/lib/mockClient.ts` returns believable demo data so the UI works out of the box.
- `src/lib/api.ts` exports `client`, defaulting to `MockClient`.
- To switch to a real backend, set `VITE_BACKEND=real` and optionally `VITE_BACKEND_URL`:

```bash
VITE_BACKEND=real VITE_BACKEND_URL=https://api.example.com pnpm dev
```

When `VITE_BACKEND=real` is set, `api.ts` instantiates `RealClient` and sends requests to `${VITE_BACKEND_URL}/search`, `/trending`, `/videos/:id`, `/streams/:id`, `/channels/:id`, `/playlists/:id`, `/sponsorblock`, and `/ryd/:id`.

## Design tokens

Colors are defined as CSS variables and mapped in `tailwind.config.js`:

| Token | Light | Dark (default) | Usage |
|-------|-------|----------------|-------|
| `bg` | `#ffffff` | `#000000` | App background (AMOLED-friendly) |
| `surface` | `#f5f5f5` | `#0f0f0f` | Cards, inputs, panels |
| `surface-hover` | `#ebebeb` | `#1c1c1c` | Hover states |
| `text` | `#0f0f0f` | `#ffffff` | Primary text |
| `subtext` | `#525252` | `#aaaaaa` | Secondary text |
| `border` | `#d4d4d4` | `#262626` | Dividers |
| `accent` | `#ff3344` | `#ff3344` | Subscribe, active states, progress |

## Persistence

- **Settings** → `localStorage` via Zustand persist.
- **History / Liked / Playlists** → IndexedDB via `idb-keyval`.
- **Downloads metadata** → IndexedDB (actual file storage intended for Tauri/SW later).

## Player features

- Custom overlaid controls: play/pause, scrub, volume, fullscreen, quality, speed, PiP.
- SponsorBlock segment markers on the scrub bar.
- Chapter tick marks.
- Audio-only mode + background audio toggle.
- Persistent mini player across all routes.

## Bundle

Production build outputs ~77 kB gzipped JS, well under the 250 kB target.
