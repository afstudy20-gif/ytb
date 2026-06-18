# `sponsorblock`

Async Rust client for the [SponsorBlock](https://sponsor.ajay.app) API.

## Features

- `segments` — direct `/skipSegments?videoID=...` lookup
- `segments_by_hash` — privacy-preserving `/skipSegments/{hashPrefix}` lookup (recommended default)
- `vote` — upvote / downvote / mark-as-no-op a segment
- `submit` — submit a new segment, returns the new UUID

## Usage

```rust
use sponsorblock::{Client, Category};

#[tokio::main]
async fn main() -> sponsorblock::Result<()> {
    let client = Client::new().with_user_id("your-private-user-hash");

    // Recommended privacy-preserving lookup
    let segments = client
        .segments_by_hash("dQw4w9WgXcQ", &[Category::Sponsor, Category::Intro])
        .await?;

    for seg in &segments {
        println!("{:?} - {}..={}", seg.category, seg.start, seg.end);
    }

    Ok(())
}
```

### Self-hosted instance

```rust
let client = sponsorblock::Client::with_base("https://sb.example.org/api");
```

### Why the hash endpoint

The default `segments_by_hash` only leaks the first 4 hex chars of
`SHA256(videoId)`. The upstream server therefore cannot tell which of the
~65 536 videos sharing that prefix the client actually wanted to skip —
only the client knows, and it filters the bucket down itself. Use it in
preference to `segments`.

## Errors

All fallible calls return [`sponsorblock::Error`], an enum covering
`Network`, `Decode`, `NotFound`, `RateLimited`, `Forbidden`, `Status` and
`InvalidInput`.

## Testing

```sh
cargo test -p sponsorblock
```

Tests are powered by [`wiremock`](https://crates.io/crates/wiremock) and
serve captured JSON fixtures from `tests/fixtures/`, so no live network is
required.
