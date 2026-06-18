//! Caching of the player JavaScript and lazy population of the cipher
//! program and n-sig function.

use std::sync::Arc;

use once_cell::sync::OnceCell;
use tokio::sync::Mutex;

use crate::client::InnerTubeClient;
use crate::error::{Error, Result};
use crate::js_interp::{CipherProgram, Interp, JsValue};
use crate::streams::extractor::{discover_player_js_url, extract_cipher_program, extract_nsig_fn, NsigFn};

/// Cached player JavaScript state. The cipher and n-sig functions only
/// change when YouTube rotates the player build, so we fetch `base.js`
/// at most once per process lifetime.
#[derive(Default)]
pub(crate) struct PlayerCache {
    /// URL of the `base.js` we last loaded.
    pub(crate) js_url: OnceCell<String>,
    /// Compiled cipher program, derived from `base.js`.
    pub(crate) cipher: OnceCell<Arc<CipherProgram>>,
    /// Parsed n-sig function (name + body), derived from `base.js`.
    pub(crate) nsig: OnceCell<Arc<NsigFn>>,
}

impl PlayerCache {
    /// Build an empty cache.
    pub(crate) fn new() -> Self {
        Self::default()
    }
}

/// Public façade wrapping the cache so it can be shared across async tasks.
pub struct PlayerJsResolver {
    http: InnerTubeClient,
    cache: Mutex<PlayerCache>,
}

impl PlayerJsResolver {
    /// Construct with a reference to the HTTP client used elsewhere.
    pub fn new(http: InnerTubeClient) -> Self {
        Self {
            http,
            cache: Mutex::new(PlayerCache::new()),
        }
    }

    /// Ensure we have a cipher program loaded. Returns an `Arc` clone so
    /// callers can hold it across awaits without locking.
    pub(crate) async fn cipher_program(&self) -> Result<Arc<CipherProgram>> {
        if let Some(cached) = self.cache.lock().await.cipher.get() {
            return Ok(cached.clone());
        }
        self.refresh().await?;
        self.cache
            .lock()
            .await
            .cipher
            .get()
            .cloned()
            .ok_or_else(|| Error::cipher("cipher program missing after refresh"))
    }

    /// Ensure we have an n-sig function loaded.
    pub(crate) async fn nsig_fn(&self) -> Result<Arc<NsigFn>> {
        if let Some(cached) = self.cache.lock().await.nsig.get() {
            return Ok(cached.clone());
        }
        self.refresh().await?;
        self.cache
            .lock()
            .await
            .nsig
            .get()
            .cloned()
            .ok_or_else(|| Error::cipher("n-sig function missing after refresh"))
    }

    /// Fetch `base.js` (caching its URL) and populate the cipher + n-sig.
    async fn refresh(&self) -> Result<()> {
        let js_url = {
            let cache = self.cache.lock().await;
            if let Some(url) = cache.js_url.get() {
                url.clone()
            } else {
                drop(cache);
                let discovered = discover_player_js_url(&self.http).await?;
                let _ = self
                    .cache
                    .lock()
                    .await
                    .js_url
                    .set(discovered.clone());
                discovered
            }
        };
        let source = self.http.get_text(&js_url).await?;
        let cipher = extract_cipher_program(&source)?;
        let nsig = extract_nsig_fn(&source)?;
        let cache = self.cache.lock().await;
        let _ = cache.cipher.set(Arc::new(cipher));
        let _ = cache.nsig.set(Arc::new(nsig));
        Ok(())
    }

    /// Apply the n-sig transform to an `n` value.
    pub(crate) async fn transform_n(&self, n: &str) -> Result<String> {
        let nsig = self.nsig_fn().await?;
        let mut interp = Interp::new();
        interp.register_function(&nsig.name, vec!["a".to_string()], nsig.body.clone());
        let value = interp.call(&nsig.name, &[JsValue::Str(n.to_string())])?;
        value.into_string()
    }

    /// Apply the cipher to an `s` value.
    pub(crate) async fn decipher_s(&self, s: &str) -> Result<String> {
        let program = self.cipher_program().await?;
        Ok(program.apply(s.to_string()))
    }
}
