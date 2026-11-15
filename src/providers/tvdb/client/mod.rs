//! TheTVDB v4 REST client.

use std::sync::Arc;

use serde::de::DeserializeOwned;
use tokio::sync::Mutex;

use super::{
    config::TvdbConfig,
    error::TvdbError,
    response::{
        Envelope, EpisodesData, ErrorBody, Links, LoginData, TvdbEpisode, TvdbMovie,
        TvdbSearchResult, TvdbSeries,
    },
};
use crate::{
    core::pagination::Page,
    providers::{http, rate_limit::RateLimiter, retry},
};

/// Results requested per `GET /search` page.
pub(crate) const SEARCH_PAGE_SIZE: u32 = 20;

/// The most episode pages [`TvdbClient::all_series_episodes`] fetches (at
/// TheTVDB's 500 episodes per page, 50 000 episodes).
const EPISODES_MAX_PAGES: u32 = 100;

/// The kind of record a [`TvdbClient::search`] is restricted to.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TvdbSearchType {
    /// Series.
    Series,
    /// Movies.
    Movie,
    /// People.
    Person,
    /// Companies.
    Company,
}

impl TvdbSearchType {
    /// The `type` query value TheTVDB expects.
    pub fn as_str(&self) -> &'static str {
        match self {
            TvdbSearchType::Series => "series",
            TvdbSearchType::Movie => "movie",
            TvdbSearchType::Person => "person",
            TvdbSearchType::Company => "company",
        }
    }
}

/// Low-level TheTVDB v4 client.
///
/// Logs in with the configured API key (and PIN) on the first request, keeps
/// the bearer token, and logs in again once when TheTVDB answers `401` —
/// tokens expire after a month. Concurrent requests share one login.
///
/// The inherent methods return TheTVDB's own records
/// ([`TvdbSeries`], [`TvdbMovie`], [`TvdbEpisode`], ...), which keep
/// TheTVDB-only data such as content ratings, every translation, and each
/// episode's absolute number. The [`SearchProvider`], [`DetailProvider`],
/// [`SeasonProvider`] and [`ExternalIdProvider`] implementations convert them
/// to the unified models.
///
/// [`SearchProvider`]: crate::unified::SearchProvider
/// [`DetailProvider`]: crate::unified::DetailProvider
/// [`SeasonProvider`]: crate::unified::SeasonProvider
/// [`ExternalIdProvider`]: crate::unified::ExternalIdProvider
///
/// # Cloning
///
/// Clones share the connection pool, the rate limiter and the bearer token.
///
/// # Example
///
/// ```no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// use cameo::providers::tvdb::{TvdbClient, TvdbConfig};
///
/// let client = TvdbClient::new(TvdbConfig::new("your-tvdb-api-key"))?;
/// let page = client.series_episodes(81189, None).await?;
/// for ep in &page.results {
///     println!("S{:02}E{:02} (#{:?})", ep.season_number, ep.number, ep.absolute_number);
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct TvdbClient {
    http: reqwest::Client,
    config: TvdbConfig,
    base_url: Arc<str>,
    rate_limiter: RateLimiter,
    token: Arc<Mutex<Option<String>>>,
}

impl std::fmt::Debug for TvdbClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TvdbClient")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl TvdbClient {
    /// Create a client from `config`. No request is made until the first call.
    ///
    /// # Errors
    ///
    /// Returns [`TvdbError::InvalidConfig`] for an empty API key or PIN, a
    /// language that is not a three-letter lowercase code, a base URL that is
    /// not an absolute http(s) URL, or an invalid rate limit; or
    /// [`TvdbError::Http`] if the HTTP client cannot be built.
    pub fn new(config: TvdbConfig) -> Result<Self, TvdbError> {
        config.validate().map_err(TvdbError::InvalidConfig)?;
        let http = http::build_client(
            config.connect_timeout,
            config.request_timeout,
            reqwest::header::HeaderMap::new(),
        )?;
        let base_url = Arc::from(config.base_url.trim_end_matches('/'));
        let rate_limiter = RateLimiter::new(config.rate_limit);
        Ok(Self {
            http,
            config,
            base_url,
            rate_limiter,
            token: Arc::new(Mutex::new(None)),
        })
    }

    /// Returns a reference to the client configuration.
    pub fn config(&self) -> &TvdbConfig {
        &self.config
    }

    // ── Endpoints ──────────────────────────────────────────────────────────────

    /// Log in now (`POST /login`) and keep the bearer token, replacing any
    /// token already held.
    ///
    /// Optional: every other method logs in on demand. Useful to check the
    /// credentials eagerly.
    ///
    /// # Errors
    ///
    /// [`TvdbError::Status`] with status `401` when TheTVDB rejects the API
    /// key or PIN.
    pub async fn login(&self) -> Result<(), TvdbError> {
        let mut slot = self.token.lock().await;
        *slot = Some(self.request_token().await?);
        Ok(())
    }

    /// Search TheTVDB (`GET /search`), optionally restricted to one `kind`.
    ///
    /// `page` is 1-indexed (default 1); each page holds up to 20 results.
    pub async fn search(
        &self,
        query: &str,
        kind: Option<TvdbSearchType>,
        page: Option<u32>,
    ) -> Result<Page<TvdbSearchResult>, TvdbError> {
        let page = page.unwrap_or(1).max(1);
        let offset = u64::from(page - 1) * u64::from(SEARCH_PAGE_SIZE);
        let mut query_pairs = vec![
            ("query", query.to_string()),
            ("limit", SEARCH_PAGE_SIZE.to_string()),
            ("offset", offset.to_string()),
        ];
        if let Some(kind) = kind {
            query_pairs.push(("type", kind.as_str().to_string()));
        }
        let envelope: Envelope<Vec<TvdbSearchResult>> = self.get("search", &query_pairs).await?;
        let links = envelope.links.unwrap_or_default();
        Ok(paged(page, envelope.data.unwrap_or_default(), &links))
    }

    /// A series with its translations, artwork, genres, remote ids, content
    /// ratings and seasons (`GET /series/{id}/extended?meta=translations`).
    ///
    /// # Errors
    ///
    /// [`TvdbError::NotFound`] when TheTVDB has no series `id`.
    pub async fn series_extended(&self, id: u64) -> Result<TvdbSeries, TvdbError> {
        let path = format!("series/{id}/extended");
        let envelope: Envelope<TvdbSeries> = self.get(&path, &extended_query()).await?;
        envelope.data.ok_or(TvdbError::NoData)
    }

    /// A movie with its translations, artwork, genres, remote ids, content
    /// ratings and companies (`GET /movies/{id}/extended?meta=translations`).
    ///
    /// # Errors
    ///
    /// [`TvdbError::NotFound`] when TheTVDB has no movie `id`.
    pub async fn movie_extended(&self, id: u64) -> Result<TvdbMovie, TvdbError> {
        let path = format!("movies/{id}/extended");
        let envelope: Envelope<TvdbMovie> = self.get(&path, &extended_query()).await?;
        envelope.data.ok_or(TvdbError::NoData)
    }

    /// One page of a series' episodes in its default order, in the configured
    /// language (`GET /series/{id}/episodes/default/{lang}`).
    ///
    /// Each [`TvdbEpisode`] carries its season, number and absolute number;
    /// specials are season `0`. `page` is 1-indexed (default 1) — TheTVDB's
    /// own pages start at 0 — and each page holds up to 500 episodes.
    pub async fn series_episodes(
        &self,
        id: u64,
        page: Option<u32>,
    ) -> Result<Page<TvdbEpisode>, TvdbError> {
        let page = page.unwrap_or(1).max(1);
        let path = format!("series/{id}/episodes/default/{}", self.config.language);
        let query = [("page", (page - 1).to_string())];
        let envelope: Envelope<EpisodesData> = self.get(&path, &query).await?;
        let links = envelope.links.unwrap_or_default();
        let episodes = envelope.data.map(|d| d.episodes).unwrap_or_default();
        Ok(paged(page, episodes, &links))
    }

    /// Every episode of a series in its default order: [`series_episodes`]
    /// for each page, concatenated (at most 100 pages).
    ///
    /// [`series_episodes`]: Self::series_episodes
    pub async fn all_series_episodes(&self, id: u64) -> Result<Vec<TvdbEpisode>, TvdbError> {
        let mut episodes = Vec::new();
        for page in 1..=EPISODES_MAX_PAGES {
            let current = self.series_episodes(id, Some(page)).await?;
            let has_next = current.has_next;
            episodes.extend(current.results);
            if !has_next {
                return Ok(episodes);
            }
        }
        tracing::warn!(
            series_id = id,
            pages = EPISODES_MAX_PAGES,
            "tvdb: stopped fetching episodes at the page cap"
        );
        Ok(episodes)
    }

    // ── Request engine ─────────────────────────────────────────────────────────

    /// `GET {base}/{path}?{query}` with the bearer token, rate limited and
    /// retried, deserializing the envelope.
    async fn get<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<Envelope<T>, TvdbError> {
        let mut url = reqwest::Url::parse(&format!("{}/{path}", self.base_url))
            .map_err(|e| TvdbError::InvalidConfig(format!("bad TheTVDB URL: {e}")))?;
        if !query.is_empty() {
            let mut pairs = url.query_pairs_mut();
            for (key, value) in query {
                pairs.append_pair(key, value);
            }
        }
        retry::run(self.config.retry, &self.rate_limiter, "tvdb", || {
            self.authorized_attempt(&url)
        })
        .await
    }

    /// One attempt: send with the held token (logging in if none), and on a
    /// `401` log in again and resend once.
    async fn authorized_attempt<T: DeserializeOwned>(
        &self,
        url: &reqwest::Url,
    ) -> Result<Envelope<T>, TvdbError> {
        let token = self.current_token().await?;
        match self.send(url, &token).await {
            Err(TvdbError::Status { status: 401, .. }) => {
                tracing::debug!("tvdb: bearer token refused; logging in again");
                let fresh = self.renew_token(&token).await?;
                self.rate_limiter.acquire().await;
                self.send(url, &fresh).await
            }
            other => other,
        }
    }

    /// Send one authorized `GET` and classify the response.
    async fn send<T: DeserializeOwned>(
        &self,
        url: &reqwest::Url,
        token: &str,
    ) -> Result<Envelope<T>, TvdbError> {
        let resp = self
            .http
            .get(url.clone())
            .bearer_auth(token)
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await?;
        decode(resp).await
    }

    /// The held bearer token, logging in first when there is none.
    async fn current_token(&self) -> Result<String, TvdbError> {
        let mut slot = self.token.lock().await;
        if let Some(token) = slot.as_ref() {
            return Ok(token.clone());
        }
        let token = self.request_token().await?;
        *slot = Some(token.clone());
        Ok(token)
    }

    /// Replace `refused` with a fresh token. When another request already
    /// renewed it, that newer token is reused rather than logging in twice.
    async fn renew_token(&self, refused: &str) -> Result<String, TvdbError> {
        let mut slot = self.token.lock().await;
        if let Some(current) = slot.as_ref()
            && current != refused
        {
            return Ok(current.clone());
        }
        *slot = None;
        let token = self.request_token().await?;
        *slot = Some(token.clone());
        Ok(token)
    }

    /// `POST /login` with the API key (and PIN), returning the bearer token.
    async fn request_token(&self) -> Result<String, TvdbError> {
        let mut body = serde_json::json!({ "apikey": self.config.api_key });
        if let Some(pin) = &self.config.pin {
            body["pin"] = serde_json::Value::String(pin.clone());
        }
        // The login is a request of its own, so it takes a rate-limit slot.
        self.rate_limiter.acquire().await;
        let resp = self
            .http
            .post(format!("{}/login", self.base_url))
            .header(reqwest::header::ACCEPT, "application/json")
            .json(&body)
            .send()
            .await?;
        let envelope: Envelope<LoginData> = decode(resp).await?;
        envelope
            .data
            .map(|d| d.token)
            .filter(|t| !t.is_empty())
            .ok_or(TvdbError::NoData)
    }
}

/// The query `…/extended` endpoints are sent with.
///
/// `short=true` is deliberately not sent: TheTVDB omits `artworks` (and
/// `characters`, `trailers`) from a short record, and the unified details
/// take their backdrop from `artworks`.
fn extended_query() -> [(&'static str, String); 1] {
    [("meta", "translations".to_string())]
}

/// Read a response: a 2xx body is the envelope; `404` is
/// [`TvdbError::NotFound`]; any other status is [`TvdbError::Status`] with
/// TheTVDB's `message` (or a truncated body).
async fn decode<T: DeserializeOwned>(resp: reqwest::Response) -> Result<Envelope<T>, TvdbError> {
    let status = resp.status();
    let retry_after = http::parse_retry_after(resp.headers());
    let text = resp.text().await?;
    if status.is_success() {
        return Ok(serde_json::from_str(&text)?);
    }
    if status == reqwest::StatusCode::NOT_FOUND {
        return Err(TvdbError::NotFound);
    }
    let message = serde_json::from_str::<ErrorBody>(&text)
        .ok()
        .and_then(|b| b.message)
        .filter(|m| !m.trim().is_empty())
        .unwrap_or_else(|| truncate_body(&text));
    Err(TvdbError::Status {
        status: status.as_u16(),
        retry_after,
        message,
    })
}

/// Build a [`Page`] from TheTVDB's `links`: `has_next` is whether a `next`
/// link is present; the totals are best-effort (saturating at `u32::MAX`).
fn paged<T>(page: u32, results: Vec<T>, links: &Links) -> Page<T> {
    let total = links.total_items.unwrap_or(0);
    let total_pages = match links.page_size {
        Some(size) if size > 0 => total.div_ceil(size),
        _ => 0,
    };
    let clamp = |n: u64| u32::try_from(n).unwrap_or(u32::MAX);
    Page::with_has_next(
        page,
        results,
        links.next.is_some(),
        clamp(total_pages),
        clamp(total),
    )
}

/// Trim a non-JSON error body to a short description.
fn truncate_body(body: &str) -> String {
    const MAX_CHARS: usize = 200;
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return "empty response body".to_string();
    }
    let truncated: String = trimmed.chars().take(MAX_CHARS).collect();
    if truncated.len() < trimmed.len() {
        format!("{truncated}…")
    } else {
        truncated
    }
}

#[cfg(test)]
mod tests;
