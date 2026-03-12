//! Spider-native web ingestion bridge.

use std::collections::HashMap;
use std::sync::Arc;

use spider::utils::clean_html;
use spider::website::Website;

use crate::llm::error::sanitize_user_visible;
use crate::llm::{LlmError, LlmResult};

/// Unified web context returned to runtime callers after one crawl operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebContext {
    /// Source URL that produced this context row.
    pub source_url: String,
    /// Best-effort document title.
    pub title: String,
    /// Best-effort markdown-like normalized body.
    pub markdown_content: Arc<str>,
    /// Transport metadata for telemetry and downstream routing.
    pub metadata: HashMap<String, String>,
}

/// Thin native bridge over `spider::Website`.
///
/// Resource hygiene policy:
/// one `Website` instance is created per ingestion and dropped immediately after
/// extraction, preventing long-lived crawler memory retention.
#[derive(Debug, Clone)]
pub struct SpiderBridge {
    root_url: Arc<str>,
    page_limit: u32,
    stealth_mode: bool,
}

impl SpiderBridge {
    /// Construct one bridge for a root URL.
    #[must_use]
    pub fn new(root_url: impl Into<String>) -> Self {
        Self {
            root_url: Arc::<str>::from(root_url.into()),
            page_limit: 1,
            stealth_mode: true,
        }
    }

    /// Set crawl page limit for quick ingestion.
    #[must_use]
    pub fn with_limit(mut self, page_limit: u32) -> Self {
        self.page_limit = page_limit.max(1);
        self
    }

    /// Enable or disable spider stealth mode hint.
    ///
    /// Note: when the crawler is built without `chrome` support, Spider treats
    /// this flag as a no-op by design.
    #[must_use]
    pub fn with_stealth(mut self, stealth_mode: bool) -> Self {
        self.stealth_mode = stealth_mode;
        self
    }

    /// Execute one non-blocking crawl pass and return normalized web context.
    ///
    /// # Errors
    ///
    /// Returns [`LlmError::Internal`] when crawl data cannot be captured from
    /// the configured URL.
    pub async fn quick_ingest(&self) -> LlmResult<WebContext> {
        let mut website = Website::new_with_firewall(self.root_url.as_ref(), false);
        website.with_limit(self.page_limit);
        website.with_stealth(self.stealth_mode);
        let mut receiver = website.subscribe(32).ok_or_else(|| {
            internal_error(format!(
                "spider ingest receiver unavailable for {}",
                self.root_url
            ))
        })?;
        website.crawl().await;
        website.unsubscribe();

        let mut page = None;
        while let Ok(candidate) = receiver.try_recv() {
            page = Some(candidate);
        }
        let page = page
            .or_else(|| website.get_pages().and_then(|pages| pages.first().cloned()))
            .ok_or_else(|| {
                internal_error(format!(
                    "spider ingest returned no pages for {}",
                    self.root_url
                ))
            })?;

        let source_url = page.get_url().to_string();
        let html = page.get_html();
        if html.trim().is_empty() {
            return Err(internal_error(format!(
                "spider ingest returned empty content for {source_url}"
            )));
        }

        let cleaned_html = clean_html(html.as_str());
        let title = page
            .metadata
            .as_deref()
            .and_then(|meta| meta.title.as_deref())
            .map_or_else(|| source_url.clone(), str::to_string);
        let markdown_content = if cleaned_html.trim().is_empty() {
            Arc::<str>::from(source_url.clone())
        } else {
            Arc::<str>::from(cleaned_html)
        };

        let mut metadata = HashMap::new();
        metadata.insert("engine".to_string(), "spider".to_string());
        metadata.insert("crawler.url".to_string(), self.root_url.to_string());
        metadata.insert(
            "crawler.page_limit".to_string(),
            self.page_limit.to_string(),
        );
        metadata.insert("crawler.stealth".to_string(), self.stealth_mode.to_string());
        if let Some(meta) = page.metadata.as_deref() {
            if let Some(description) = meta.description.as_deref() {
                metadata.insert("page.description".to_string(), description.to_string());
            }
            if let Some(image) = meta.image.as_deref() {
                metadata.insert("page.image".to_string(), image.to_string());
            }
        }

        Ok(WebContext {
            source_url,
            title,
            markdown_content,
            metadata,
        })
    }
}

fn internal_error(message: impl Into<String>) -> LlmError {
    LlmError::Internal {
        message: sanitize_user_visible(message.into().as_str()),
    }
}
