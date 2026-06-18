//! Continuation endpoint: fetch the next page of a paginated response
//! (search results, playlist entries, related videos).
//!
//! The InnerTube `search`, `browse`, and `next` endpoints all share a
//! continuation mechanism: each paginated response includes a
//! `continuationItemRenderer` whose `token` can be POSTed back to the same
//! endpoint to fetch the next page. The shape of the *response* is similar
//! to the original but with most of the surrounding metadata stripped.

#![allow(dead_code)]

use serde_json::{Map, Value};

use crate::client::{ClientContext, InnerTubeClient};
use crate::error::Result;
use crate::search::parse_search_response;
use crate::types::search::SearchResults;

/// Issue a `search` continuation call.
pub(crate) async fn search_continuation(
    http: &InnerTubeClient,
    token: &str,
) -> Result<SearchResults> {
    let mut body = Map::new();
    body.insert("continuation".into(), Value::String(token.to_string()));
    let resp = http.post("search", ClientContext::WEB_DEFAULT, body).await?;
    parse_search_response(&resp)
}

/// Issue a `browse` continuation call. The caller is responsible for
/// re-parsing the response into the appropriate target type (channel
/// videos, playlist entries, etc.).
pub(crate) async fn browse_continuation_raw(
    http: &InnerTubeClient,
    token: &str,
) -> Result<Value> {
    let mut body = Map::new();
    body.insert("continuation".into(), Value::String(token.to_string()));
    http.post("browse", ClientContext::WEB_DEFAULT, body).await
}
