# `ryd`

Async Rust client for the [Return YouTube Dislike](https://returnyoutubedislike.com)
API, with a built-in in-memory LRU cache (256 entries, 5-minute TTL by
default) to stay friendly to the upstream service.

## Usage

```rust
use ryd::Client;
use std::time::Duration;

#[tokio::main]
async fn main() -> ryd::Result<()> {
    let client = Client::new();
    let votes = client.votes("dQw4w9WgXcQ").await?;
    println!("{} likes / {} dislikes", votes.likes, votes.dislikes);
    Ok(())
}
```

### Self-hosted instance / custom cache

```rust
let client = ryd::Client::new()
    .with_base("https://ryd.example.org")
    .with_cache(512, Duration::from_secs(60));
```

The builder methods are chainable: `with_base`, `with_cache`, `with_http`.

## Caching behaviour

- Successful, non-deleted responses are cached for the configured TTL.
- Records marked `deleted: true` upstream are **not** cached — they may
  reflect transient upstream state and should be re-fetched.
- The LRU evicts the least-recently-used entry when capacity is reached.

## Errors

All fallible calls return [`ryd::Error`], an enum covering `Network`,
`Decode`, `NotFound`, `RateLimited`, `Status` and `InvalidInput`.

## Testing

```sh
cargo test -p ryd
```

Tests use [`wiremock`](https://crates.io/crates/wiremock) against the
captured fixture in `tests/fixtures/votes.json`, so no live network is
required.
