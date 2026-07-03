use futures::Stream;
use serde::{Deserialize, Serialize};

/// How to fetch the page following a [`Page`].
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PageToken {
    /// A 1-indexed page number (offset pagination).
    Page(u32),
    /// An opaque provider cursor (cursor pagination).
    Cursor(String),
}

/// A single page of results from a provider.
///
/// Provider-agnostic per ADR 0001: `has_next` is **authoritative** (never
/// reconstructed from nullable totals), `next` carries how to fetch the
/// following page (page number or cursor), and `total_*` are best-effort
/// (`0` when the provider does not report them — rely on `has_next` for
/// iteration, not the totals).
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Page<T> {
    /// Current page number (1-indexed) where meaningful.
    pub page: u32,
    /// Items on this page.
    pub results: Vec<T>,
    /// Whether more pages follow. Authoritative — set from the provider's own
    /// has-next signal, not derived from totals.
    pub has_next: bool,
    /// How to fetch the next page, if any (page number or opaque cursor).
    pub next: Option<PageToken>,
    /// Total number of pages (best-effort; `0` if unknown).
    pub total_pages: u32,
    /// Total number of results across all pages (best-effort; `0` if unknown).
    pub total_results: u32,
}

impl<T> Page<T> {
    /// Build a page from **offset** pagination metadata, deriving `has_next`
    /// from `page < total_pages`.
    pub fn offset(page: u32, results: Vec<T>, total_pages: u32, total_results: u32) -> Self {
        let has_next = page < total_pages;
        Self {
            page,
            results,
            has_next,
            next: has_next.then(|| PageToken::Page(page.saturating_add(1))),
            total_pages,
            total_results,
        }
    }

    /// Build a page with an **authoritative** `has_next` (e.g. AniList's
    /// `hasNextPage`), with best-effort totals.
    pub fn with_has_next(
        page: u32,
        results: Vec<T>,
        has_next: bool,
        total_pages: u32,
        total_results: u32,
    ) -> Self {
        Self {
            page,
            results,
            has_next,
            next: has_next.then(|| PageToken::Page(page.saturating_add(1))),
            total_pages,
            total_results,
        }
    }

    /// Returns `true` if there are more pages after this one.
    pub fn has_next_page(&self) -> bool {
        self.has_next
    }

    /// The next page number, if paginating by offset.
    pub fn next_page(&self) -> Option<u32> {
        match &self.next {
            Some(PageToken::Page(p)) => Some(*p),
            Some(PageToken::Cursor(_)) => None,
            None => None,
        }
    }

    /// Map the items of this page, preserving pagination metadata.
    pub fn map<U, F: FnMut(T) -> U>(self, f: F) -> Page<U> {
        Page {
            page: self.page,
            results: self.results.into_iter().map(f).collect(),
            has_next: self.has_next,
            next: self.next,
            total_pages: self.total_pages,
            total_results: self.total_results,
        }
    }

    /// Transform each item, dropping items the closure maps to `None`.
    ///
    /// Pagination metadata (page numbers and totals) is preserved from the
    /// provider — the server's totals still describe the full result set even
    /// when some items on this page are filtered out locally.
    pub fn filter_map<U, F: FnMut(T) -> Option<U>>(self, f: F) -> Page<U> {
        Page {
            page: self.page,
            results: self.results.into_iter().filter_map(f).collect(),
            has_next: self.has_next,
            next: self.next,
            total_pages: self.total_pages,
            total_results: self.total_results,
        }
    }
}

/// Converts a page-fetching closure into an async [`Stream`] that yields
/// individual items across all pages.
///
/// `fetch_page` receives a 1-indexed page number and returns the items on that
/// page along with pagination metadata. The returned stream is `Send`, so it can
/// cross threads and be `tokio::spawn`ed on a multi-thread runtime.
///
/// # Examples
///
/// ```rust,no_run
/// use cameo::core::pagination::{into_stream, Page};
/// use futures::StreamExt;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let stream = into_stream(|page: u32| async move {
///     Ok::<Page<String>, std::convert::Infallible>(Page::offset(
///         page,
///         vec!["Inception".to_string()],
///         1,
///         1,
///     ))
/// });
///
/// tokio::pin!(stream);
/// while let Some(item) = stream.next().await {
///     println!("{}", item?);
/// }
/// # Ok(())
/// # }
/// ```
pub fn into_stream<T, E, F, Fut>(fetch_page: F) -> impl Stream<Item = Result<T, E>> + Send
where
    T: Send + 'static,
    E: Send + 'static,
    F: Fn(u32) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<Page<T>, E>> + Send + 'static,
{
    async_stream::try_stream! {
        let mut page = 1u32;
        loop {
            let response = fetch_page(page).await?;
            let has_next = response.has_next_page();
            let next = response.next_page();
            for item in response.results {
                yield item;
            }
            match next {
                Some(n) if has_next => page = n,
                _ => break,
            }
        }
    }
}
