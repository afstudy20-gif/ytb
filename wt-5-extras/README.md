# wt-5-extras

Monorepo of auxiliary integrations that the main **wt-5** YouTube client plugs into:

| Crate / package            | Language | Purpose                                                                |
| -------------------------- | -------- | ---------------------------------------------------------------------- |
| `crates/sponsorblock`      | Rust     | SponsorBlock client (segments, voting, submission, hash-prefix lookups)|
| `crates/ryd`               | Rust     | Return YouTube Dislike client with in-memory LRU cache                  |
| `packages/sponsorblock-web`| TypeScript | Browser mirror of the Rust SponsorBlock client (fetch-based)         |
| `packages/ryd-web`         | TypeScript | Browser mirror of the Rust RYD client (fetch-based, cached)          |
| `packages/pwa-shell`       | TypeScript | PWA primitives: service worker, MediaSession background-audio, IndexedDB offline store |

## Layout

```
wt-5-extras/
  Cargo.toml          # Rust workspace (crates/sponsorblock, crates/ryd)
  package.json        # pnpm workspace root
  pnpm-workspace.yaml
  tsconfig.base.json
  crates/
    sponsorblock/
    ryd/
  packages/
    sponsorblock-web/
    ryd-web/
    pwa-shell/
```

## How it slots into the main app

- **SponsorBlock** — Rust crate is used by the Tauri/desktop backend to skip sponsored segments; the TypeScript mirror is used by the web build. Both expose an identical request surface so player code can be shared.
- **Return YouTube Dislike** — Provides dislike counts for the UI. Rust for desktop, TS for web. Both implement a 5-minute TTL cache to stay friendly to the upstream API.
- **PWA shell** — Imported by the web build only. Registers a hand-rolled service worker (cache-first static / network-first API / IndexedDB-backed segment cache), wires `navigator.mediaSession` to an `<audio>` element so playback survives app backgrounding, and stores downloaded media blobs in IndexedDB.

## Toolchain

- Rust 1.75+ (workspace uses edition 2021, resolver 2)
- Node 20+
- pnpm 9+ (workspace declared via `pnpm-workspace.yaml`)

## Verification

```bash
# Rust side
cargo check
cargo clippy --all-targets -- -D warnings
cargo test

# TypeScript side
pnpm install
pnpm -r typecheck
pnpm -r build
pnpm -r test
```

See each package's `README.md` for usage details.
