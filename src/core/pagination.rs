use futures::Stream;
use serde::{Deserialize, Serialize};

/// A paginated response from a provider API.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    /// Current page number (1-indexed).
    pub page: u32,
    /// Items on this page.
    pub results: Vec<T>,
    /// Total number of pages available.
    pub total_pages: u32,
    /// Total number of results across all pages.
    pub total_results: u32,
}

impl<T> PaginatedResponse<T> {
    /// Create a new `PaginatedResponse`.
    pub fn new(page: u32, results: Vec<T>, total_pages: u32, total_results: u32) -> Self {
        Self {
            page,
            results,
            total_pages,
            total_results,
        }
    }

    /// Returns `true` if there are more pages after this one.
    pub fn has_next_page(&self) -> bool {
        self.page < self.total_pages
    }

    /// Returns the next page number, or `None` if this is the last page.
    pub fn next_page(&self) -> Option<u32> {
        if self.has_next_page() {
            Some(self.page + 1)
        } else {
            None
        }
    }
}

/// Converts a page-fetching closure into an async [`Stream`] that yields
/// individual items across all pages.
///
/// `fetch_page` receives a 1-indexed page number and returns the items on that page
/// along with pagination metadata.
///
/// # Examples
///
/// ```rust,no_run
/// use cameo::core::pagination::{into_stream, PaginatedResponse};
/// use futures::StreamExt;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // `fetch_page` is any async closure that takes a 1-indexed page number
/// // and returns a PaginatedResponse.  Replace the body with a real client call.
/// let stream = into_stream(|page: u32| async move {
///     // e.g. client.search_movies("Inception", Some(page)).await
///     Ok::<PaginatedResponse<String>, std::convert::Infallible>(PaginatedResponse::new(
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
pub fn into_stream<T, E, F, Fut>(fetch_page: F) -> impl Stream<Item = Result<T, E>>
where
    T: 'static,
    E: 'static,
    F: Fn(u32) -> Fut + 'static,
    Fut: std::future::Future<Output = Result<PaginatedResponse<T>, E>> + 'static,
{
    async_stream::try_stream! {
        let mut page = 1u32;
        loop {
            let response = fetch_page(page).await?;
            let has_next = response.has_next_page();
            for item in response.results {
                yield item;
            }
            if !has_next {
                break;
            }
            page += 1;
        }
    }
}
