//! The GraphQL transport: one request per attempt, paced by the rate limiter,
//! retried by the shared retry loop, and classified into a root or an error.

use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use super::AniListClient;
use crate::{
    core::error::DecodeError,
    providers::{
        anilist::{error::AniListError, response::GraphQlResponse},
        http, retry,
    },
};

impl AniListClient {
    /// Execute a GraphQL query and extract the root field the caller needs.
    ///
    /// `root` projects the deserialized `data` object onto the root field the
    /// caller needs (e.g. `Media` or `Page`). Classification happens per
    /// attempt, so a transient upstream failure that nulls the root field is
    /// retried like any other 429/5xx.
    ///
    /// The response body is read **before** the HTTP status is inspected, so
    /// GraphQL errors and rate-limit headers are surfaced for every status
    /// (AniList returns `404`/`429` as GraphQL errors on an otherwise-2xx
    /// response, and rate-limit info in `Retry-After`).
    ///
    /// # Error classification
    ///
    /// - A GraphQL error carrying `status: 404` always maps to
    ///   [`AniListError::NotFound`].
    /// - **Partial data:** when the response carries errors but the root field
    ///   is non-null, the root is returned and the errors are logged at `warn`
    ///   level rather than discarded — failing would throw away good data.
    /// - When the response carries errors and the root field is null or
    ///   missing, the errors are surfaced as [`AniListError::GraphQL`] (never
    ///   as not-found / no-data), retried when any status is 429/5xx.
    ///   This holds even when the errors carry only a non-error status (e.g.
    ///   `200`): the server reported errors, so we do not claim the item is
    ///   missing.
    /// - If the HTTP status is non-2xx, GraphQL errors without a `status`
    ///   inherit the HTTP status (before any other check), so e.g. an HTTP 500
    ///   with status-less errors is treated as a 5xx and an HTTP 404 as
    ///   not-found.
    /// - With no errors (absent or empty) and a null root, a non-2xx HTTP
    ///   status yields [`AniListError::Status`] (retried on 429/5xx).
    /// - With no errors, a null root on a 2xx response yields `Ok(None)`; the
    ///   caller decides whether that means not-found (details) or no-data
    ///   (lists).
    pub(super) async fn graphql<R, T>(
        &self,
        query: &str,
        variables: Value,
        root: fn(R) -> Option<T>,
    ) -> Result<Option<T>, AniListError>
    where
        R: DeserializeOwned,
    {
        let body = json!({
            "query": query,
            "variables": variables,
        });

        // Pace requests (including retries) within the rate limit.
        retry::run(self.config.retry, &self.rate_limiter, "anilist", || {
            self.graphql_attempt(&body, root)
        })
        .await
    }

    /// A single GraphQL request attempt (no rate limiting or retrying).
    ///
    /// See [`graphql`](Self::graphql) for the classification rules.
    async fn graphql_attempt<R, T>(
        &self,
        body: &Value,
        root: fn(R) -> Option<T>,
    ) -> Result<Option<T>, AniListError>
    where
        R: DeserializeOwned,
    {
        let resp = self
            .http
            .post(&self.config.base_url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(reqwest::header::ACCEPT, "application/json")
            .json(body)
            .send()
            .await
            .map_err(AniListError::from_reqwest)?;

        // Capture status and rate-limit headers before consuming the body.
        let status = resp.status();
        let retry_after = http::parse_retry_after(resp.headers());
        let text = resp.text().await.map_err(AniListError::from_reqwest)?;

        // Parse the GraphQL envelope regardless of HTTP status.
        let gql_resp: GraphQlResponse<R> = match serde_json::from_str(&text) {
            Ok(parsed) => parsed,
            Err(parse_err) => {
                // The body was not a GraphQL envelope. An error status is the
                // more useful explanation; otherwise it is a real decode error.
                if !status.is_success() {
                    return Err(AniListError::Status {
                        status: status.as_u16(),
                        retry_after,
                        message: truncate_body(&text),
                    });
                }
                return Err(AniListError::Deserialization(DecodeError::json(parse_err)));
            }
        };

        if let Some(mut errors) = gql_resp.errors.filter(|e| !e.is_empty()) {
            // Status-less errors inherit a non-2xx HTTP status so they
            // classify correctly (e.g. HTTP 404 → not found, HTTP 500 → 5xx).
            if !status.is_success() {
                let http_status = i64::from(status.as_u16());
                for e in errors.iter_mut().filter(|e| e.status.is_none()) {
                    e.status = Some(http_status);
                }
            }
            // Derive not-found from the structured `status`, not the message.
            if errors.iter().any(|e| e.status == Some(404)) {
                return Err(AniListError::NotFound);
            }
            // Partial-data policy: prefer a usable root over failing.
            if let Some(value) = gql_resp.data.and_then(root) {
                tracing::warn!(?errors, "anilist: returning partial data alongside errors");
                return Ok(Some(value));
            }
            // The root is null/missing: the errors explain why.
            tracing::warn!(?errors, "anilist: graphql errors");
            return Err(AniListError::GraphQL {
                errors,
                retry_after,
            });
        }

        // No errors (absent or empty).
        let data_present = gql_resp.data.is_some();
        if let Some(value) = gql_resp.data.and_then(root) {
            return Ok(Some(value));
        }
        // Null root / no data on an error status: report the HTTP status.
        if !status.is_success() {
            return Err(AniListError::Status {
                status: status.as_u16(),
                retry_after,
                message: truncate_body(&text),
            });
        }
        if data_present {
            // A successful response with a null root: the caller decides.
            Ok(None)
        } else {
            Err(AniListError::NoData)
        }
    }
}

/// Truncate a response body to a bounded length for use in an error message.
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
mod tests {
    use super::truncate_body;

    #[test]
    fn truncate_body_trims_surrounding_whitespace() {
        assert_eq!(truncate_body("  \n body text \t\n"), "body text");
    }

    #[test]
    fn truncate_body_names_an_empty_body() {
        assert_eq!(truncate_body(""), "empty response body");
        assert_eq!(truncate_body(" \n\t "), "empty response body");
    }

    #[test]
    fn truncate_body_keeps_200_chars_and_marks_the_cut() {
        let exact = "a".repeat(200);
        assert_eq!(truncate_body(&exact), exact);
        assert_eq!(
            truncate_body(&"a".repeat(201)),
            format!("{}…", "a".repeat(200))
        );
        // Trimming happens before counting.
        assert_eq!(truncate_body(&format!("   {exact}   ")), exact);
    }

    #[test]
    fn truncate_body_counts_chars_not_bytes() {
        let exact = "é".repeat(200);
        assert_eq!(truncate_body(&exact), exact);
        assert_eq!(
            truncate_body(&"é".repeat(201)),
            format!("{}…", "é".repeat(200))
        );
    }
}
