//! Spider-native web ingestion bridge.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use spider::features::chrome_common::RequestInterceptConfiguration;
use spider::utils::clean_html;
use spider::website::Website;

use crate::llm::error::sanitize_user_visible;
use crate::llm::{LlmError, LlmResult};
use crate::web::spider_config;

const DEFAULT_SPIDER_USER_AGENTS: [&str; 3] = [
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:133.0) Gecko/20100101 Firefox/133.0",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.1 Safari/605.1.15",
];

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
    user_agent: Option<Arc<str>>,
    chrome_intercept: Option<bool>,
    prefer_raw_html_on_clean_empty: Option<bool>,
}

impl SpiderBridge {
    /// Construct one bridge for a root URL.
    #[must_use]
    pub fn new(root_url: impl Into<String>) -> Self {
        Self {
            root_url: Arc::<str>::from(root_url.into()),
            page_limit: 1,
            stealth_mode: true,
            user_agent: None,
            chrome_intercept: None,
            prefer_raw_html_on_clean_empty: None,
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

    /// Override HTTP `User-Agent` for this crawl call.
    #[must_use]
    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        let value = user_agent.into();
        let trimmed = value.trim();
        self.user_agent = if trimmed.is_empty() {
            None
        } else {
            Some(Arc::<str>::from(trimmed.to_string()))
        };
        self
    }

    /// Toggle Spider chrome request intercept mode.
    ///
    /// This is best effort; when Spider is built without `chrome`, Spider treats
    /// this setting as a no-op.
    #[must_use]
    pub fn with_chrome_intercept(mut self, enabled: bool) -> Self {
        self.chrome_intercept = Some(enabled);
        self
    }

    /// Control fallback behavior when `clean_html` returns empty text.
    ///
    /// `true`: fallback to raw HTML content before URL fallback.
    /// `false`: fallback directly to URL marker.
    #[must_use]
    pub fn with_prefer_raw_html_on_clean_empty(mut self, enabled: bool) -> Self {
        self.prefer_raw_html_on_clean_empty = Some(enabled);
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
        let user_agent = self.resolve_user_agent();
        website.with_user_agent(Some(user_agent.as_ref()));
        let chrome_intercept = self.resolve_chrome_intercept();
        if chrome_intercept {
            website.with_chrome_intercept(RequestInterceptConfiguration::new(true));
        }
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
        let prefer_raw_html_on_clean_empty = self.resolve_prefer_raw_html_on_clean_empty();
        let (markdown_content, content_source) = resolve_markdown_content(
            cleaned_html.as_str(),
            html.as_str(),
            source_url.as_str(),
            prefer_raw_html_on_clean_empty,
        );
        let title = page
            .metadata
            .as_deref()
            .and_then(|meta| meta.title.as_deref())
            .map_or_else(|| source_url.clone(), str::to_string);

        let mut metadata = HashMap::new();
        metadata.insert("engine".to_string(), "spider".to_string());
        metadata.insert("crawler.url".to_string(), self.root_url.to_string());
        metadata.insert(
            "crawler.page_limit".to_string(),
            self.page_limit.to_string(),
        );
        metadata.insert("crawler.stealth".to_string(), self.stealth_mode.to_string());
        metadata.insert("crawler.user_agent".to_string(), user_agent.to_string());
        metadata.insert(
            "crawler.chrome_intercept".to_string(),
            chrome_intercept.to_string(),
        );
        metadata.insert(
            "crawler.prefer_raw_html_on_clean_empty".to_string(),
            prefer_raw_html_on_clean_empty.to_string(),
        );
        metadata.insert(
            "crawler.content_source".to_string(),
            content_source.to_string(),
        );
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

impl SpiderBridge {
    fn resolve_user_agent(&self) -> Arc<str> {
        if let Some(explicit) = self.user_agent.as_ref() {
            return explicit.clone();
        }
        if let Some(env_value) = non_empty_env("XIUXIAN_LLM_SPIDER_USER_AGENT")
            .or_else(|| non_empty_env("XIUXIAN_SPIDER_USER_AGENT"))
        {
            return Arc::<str>::from(env_value);
        }
        if let Some(config_value) = spider_config::user_agent() {
            return Arc::<str>::from(config_value);
        }
        Arc::<str>::from(default_user_agent_for_url(self.root_url.as_ref()))
    }

    fn resolve_chrome_intercept(&self) -> bool {
        self.chrome_intercept
            .or_else(|| parse_bool_env("XIUXIAN_LLM_SPIDER_CHROME_INTERCEPT"))
            .or_else(|| parse_bool_env("XIUXIAN_SPIDER_CHROME_INTERCEPT"))
            .or_else(spider_config::chrome_intercept)
            .unwrap_or(false)
    }

    fn resolve_prefer_raw_html_on_clean_empty(&self) -> bool {
        self.prefer_raw_html_on_clean_empty
            .or_else(|| parse_bool_env("XIUXIAN_LLM_SPIDER_PREFER_RAW_HTML_ON_CLEAN_EMPTY"))
            .or_else(|| parse_bool_env("XIUXIAN_SPIDER_PREFER_RAW_HTML_ON_CLEAN_EMPTY"))
            .or_else(spider_config::prefer_raw_html_on_clean_empty)
            .unwrap_or(true)
    }
}

pub(super) fn resolve_markdown_content(
    cleaned_html: &str,
    raw_html: &str,
    source_url: &str,
    prefer_raw_html_on_clean_empty: bool,
) -> (Arc<str>, &'static str) {
    if !cleaned_html.trim().is_empty() {
        return (Arc::<str>::from(cleaned_html.to_string()), "clean_html");
    }
    if prefer_raw_html_on_clean_empty && !raw_html.trim().is_empty() {
        return (Arc::<str>::from(raw_html.to_string()), "raw_html");
    }
    (Arc::<str>::from(source_url.to_string()), "url_fallback")
}

fn default_user_agent_for_url(url: &str) -> &'static str {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    url.hash(&mut hasher);
    let pool_len = DEFAULT_SPIDER_USER_AGENTS.len();
    if pool_len == 0 {
        return "Mozilla/5.0";
    }
    let modulo = hasher.finish() % u64::try_from(pool_len).unwrap_or(1);
    let idx = usize::try_from(modulo).unwrap_or(0);
    DEFAULT_SPIDER_USER_AGENTS[idx]
}

fn non_empty_env(key: &str) -> Option<String> {
    std::env::var(key).ok().and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn parse_bool_env(key: &str) -> Option<bool> {
    non_empty_env(key).and_then(|value| match value.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    })
}
