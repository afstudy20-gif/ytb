//! Tiny bounded LRU cache with per-entry TTL.
//!
//! Hand-rolled to avoid pulling in `lru` / `moka` for what is a very small
//! lookup table (256 entries, 5-minute TTL by default). Layout: an access-order
//! `VecDeque` of `(key, Entry)` pairs with a `HashMap<String, usize>` index
//! pointing at each pair's slot, all behind a `std::sync::Mutex`.
//!
//! Because the table is capped at a few hundred entries, a linear move-to-tail
//! on access is cheap and avoids the index-bookkeeping pitfalls of a manual
//! doubly-linked-list-in-a-Vec.

use crate::model::Votes;
use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Default capacity — matches the spec's "256 entries".
pub const DEFAULT_CAPACITY: usize = 256;
/// Default TTL — matches the spec's "5 minutes".
pub const DEFAULT_TTL: Duration = Duration::from_secs(5 * 60);

struct Entry {
    votes: Votes,
    expires_at: Instant,
}

pub(crate) struct Lru {
    inner: Mutex<Inner>,
}

struct Inner {
    ttl: Duration,
    /// `(key, entry)` pairs; the head is the least-recently-used.
    order: VecDeque<(String, Entry)>,
    /// Key -> index into `order`, kept in sync on every mutation.
    index: HashMap<String, usize>,
}

impl Inner {
    fn new(cap: usize, ttl: Duration) -> Self {
        Self {
            ttl,
            order: VecDeque::with_capacity(cap.max(1)),
            index: HashMap::with_capacity(cap.max(1)),
        }
    }

    fn cap(&self) -> usize {
        // capacity is enforced by the caller; store nothing extra here.
        self.order.capacity().min(usize::MAX)
    }
}

impl Lru {
    pub(crate) fn new(cap: usize, ttl: Duration) -> Self {
        let cap = cap.max(1);
        let mut inner = Inner::new(cap, ttl);
        // Reserve exact capacity so push_back never grows past it.
        inner.order = VecDeque::with_capacity(cap);
        inner.index = HashMap::with_capacity(cap);
        // Stash the intended capacity as the order deque's reserved capacity;
        // `cap()` reads it back.
        Self {
            inner: Mutex::new(inner),
        }
    }

    /// Look up a key, returning `None` when missing or stale. A hit promotes
    /// the entry to the most-recently-used position (tail).
    pub(crate) fn get(&self, key: &str) -> Option<Votes> {
        let mut inner = self.inner.lock().ok()?;
        let now = Instant::now();
        let idx = *inner.index.get(key)?;
        // Read expiry by cloning the needed bits, then decide.
        let (votes, expired) = match inner.order.get(idx) {
            Some((_, e)) => (e.votes.clone(), e.expires_at <= now),
            None => return None,
        };
        if expired {
            remove_at(&mut inner, idx);
            return None;
        }
        promote_to_tail(&mut inner, key);
        Some(votes)
    }

    /// Insert / replace an entry, evicting the LRU element if at capacity.
    pub(crate) fn put(&self, key: String, votes: Votes) {
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        let cap = inner.cap();
        let ttl = inner.ttl;
        let expires_at = Instant::now() + ttl;

        // Refresh existing entry.
        if let Some(&idx) = inner.index.get(&key) {
            if let Some((_, entry)) = inner.order.get_mut(idx) {
                entry.votes = votes;
                entry.expires_at = expires_at;
            }
            promote_to_tail(&mut inner, &key);
            return;
        }

        // Evict from head (LRU) until we are below cap.
        while inner.order.len() >= cap {
            if let Some((victim, _)) = inner.order.pop_front() {
                inner.index.remove(&victim);
            } else {
                break;
            }
        }
        reindex(&mut inner);

        let new_idx = inner.order.len();
        inner.order.push_back((
            key.clone(),
            Entry {
                votes,
                expires_at,
            },
        ));
        inner.index.insert(key, new_idx);
    }

    /// Number of entries currently stored (including not-yet-reaped stale ones).
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.inner.lock().map(|i| i.order.len()).unwrap_or(0)
    }

    #[cfg(test)]
    pub(crate) fn clear(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.order.clear();
            inner.index.clear();
        }
    }
}

/// Move the entry identified by `key` to the tail (MRU) and re-index.
fn promote_to_tail(inner: &mut Inner, key: &str) {
    let Some(&idx) = inner.index.get(key) else {
        return;
    };
    if idx == inner.order.len().saturating_sub(1) {
        return; // already MRU
    }
    let entry = inner.order.remove(idx);
    reindex(inner);
    if let Some(pair) = entry {
        inner.order.push_back(pair);
        reindex(inner);
    }
}

/// Drop the entry at `idx` and re-index the rest.
fn remove_at(inner: &mut Inner, idx: usize) {
    if inner.order.remove(idx).is_some() {
        reindex(inner);
    }
}

/// Rebuild the key->index map from the order deque.
fn reindex(inner: &mut Inner) {
    inner.index.clear();
    inner.index.reserve(inner.order.len());
    for (i, (k, _)) in inner.order.iter().enumerate() {
        inner.index.insert(k.clone(), i);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vote(id: &str, likes: i64) -> Votes {
        Votes {
            id: id.to_string(),
            date_created: 0.0,
            likes,
            dislikes: 0,
            rating: 5.0,
            view_count: 0,
            deleted: false,
        }
    }

    #[test]
    fn put_and_get() {
        let lru = Lru::new(2, Duration::from_secs(60));
        lru.put("a".into(), vote("a", 1));
        assert_eq!(lru.get("a").map(|v| v.likes), Some(1));
    }

    #[test]
    fn evicts_lru_on_capacity() {
        let lru = Lru::new(2, Duration::from_secs(60));
        lru.put("a".into(), vote("a", 1));
        lru.put("b".into(), vote("b", 2));
        // Touch a so b becomes LRU.
        let _ = lru.get("a");
        lru.put("c".into(), vote("c", 3));
        assert!(lru.get("b").is_none(), "b should have been evicted");
        assert!(lru.get("a").is_some());
        assert!(lru.get("c").is_some());
    }

    #[test]
    fn ttl_expires() {
        let lru = Lru::new(8, Duration::from_millis(10));
        lru.put("a".into(), vote("a", 1));
        std::thread::sleep(Duration::from_millis(20));
        assert!(lru.get("a").is_none());
    }

    #[test]
    fn refresh_does_not_grow_beyond_cap() {
        let lru = Lru::new(2, Duration::from_secs(60));
        lru.put("a".into(), vote("a", 1));
        lru.put("a".into(), vote("a", 2));
        lru.put("b".into(), vote("b", 9));
        assert_eq!(lru.len(), 2);
    }

    #[test]
    fn refresh_updates_value() {
        let lru = Lru::new(4, Duration::from_secs(60));
        lru.put("a".into(), vote("a", 1));
        lru.put("a".into(), vote("a", 99));
        assert_eq!(lru.get("a").map(|v| v.likes), Some(99));
    }
}
