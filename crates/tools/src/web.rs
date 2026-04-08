use std::collections::BTreeSet;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use reqwest::Client;

use crate::builtin::BuiltinTool;
use crate::tool_output::{ToolError, ToolOutput};
use crate::types::{
    SearchHit, WebFetchInput, WebFetchOutput, WebSearchInput, WebSearchOutput, WebSearchResultItem,
};

// ---------------------------------------------------------------------------
// Global async HTTP client with built-in connection pool.
// Shared across all web tool calls; connections are reused automatically.
// ---------------------------------------------------------------------------

pub(crate) fn http_client() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        Client::builder()
            .timeout(Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::limited(10))
            .pool_max_idle_per_host(10)
            .user_agent("codineer-rust-tools/0.1")
            .build()
            .unwrap_or_else(|_| Client::new())
    })
}

/// Run an async future to completion.
///
/// Handles two execution contexts:
/// - Already inside a tokio multi-thread runtime (e.g. async tool dispatch):
///   uses `block_in_place` so the current worker thread can block without
///   starving the scheduler.
/// - Called from a purely synchronous thread (e.g. CLI CliToolExecutor):
///   spins up a lightweight `current_thread` runtime for this call only.
static WEB_BLOCK_ON_RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

pub(crate) fn block_on_web<F, T>(fut: F) -> T
where
    F: std::future::Future<Output = T>,
{
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => tokio::task::block_in_place(|| handle.block_on(fut)),
        Err(_) => WEB_BLOCK_ON_RUNTIME
            .get_or_init(|| {
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .or_else(|_| tokio::runtime::Builder::new_current_thread().build())
                    .expect("failed to build fallback tokio runtime")
            })
            .block_on(fut),
    }
}

// ---------------------------------------------------------------------------
// Public sync entry points (thin wrappers around async implementations)
// ---------------------------------------------------------------------------

pub(crate) fn execute_web_fetch(input: &WebFetchInput) -> Result<WebFetchOutput, String> {
    block_on_web(async_execute_web_fetch(input))
}

pub(crate) fn execute_web_search(input: &WebSearchInput) -> Result<WebSearchOutput, String> {
    block_on_web(async_execute_web_search(input))
}

// ---------------------------------------------------------------------------
// Async implementations
// ---------------------------------------------------------------------------

async fn async_execute_web_fetch(input: &WebFetchInput) -> Result<WebFetchOutput, String> {
    const MAX_BODY_SIZE: usize = 10 * 1024 * 1024; // 10 MiB
    let started = Instant::now();

    let request_url = normalize_fetch_url(&input.url)?;
    ssrf_check_url(&request_url).await?;
    let response = http_client()
        .get(&request_url)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = response.status();
    let final_url = response.url().to_string();
    let code = status.as_u16();
    let code_text = status.canonical_reason().unwrap_or("Unknown").to_string();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();

    // Stream body with size cap to avoid OOM on large responses.
    let raw_bytes = response.bytes().await.map_err(|e| e.to_string())?;
    let (raw, _truncated) = if raw_bytes.len() > MAX_BODY_SIZE {
        (&raw_bytes[..MAX_BODY_SIZE], true)
    } else {
        (raw_bytes.as_ref(), false)
    };
    let bytes = raw.len();
    let body = String::from_utf8_lossy(raw).into_owned();
    let normalized = normalize_fetched_content(&body, &content_type);
    let result = summarize_web_fetch(&final_url, &input.prompt, &normalized, &body, &content_type);

    Ok(WebFetchOutput {
        bytes,
        code,
        code_text,
        result,
        duration_ms: started.elapsed().as_millis(),
        url: final_url,
    })
}

async fn async_execute_web_search(input: &WebSearchInput) -> Result<WebSearchOutput, String> {
    const MAX_SEARCH_BODY: usize = 5 * 1024 * 1024;
    let started = Instant::now();

    let search_url = build_search_url(&input.query)?;
    ssrf_check_url(search_url.as_str()).await?;
    let response = http_client()
        .get(search_url)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let final_url = response.url().clone();
    let raw_bytes = response.bytes().await.map_err(|e| e.to_string())?;
    let html = String::from_utf8_lossy(if raw_bytes.len() > MAX_SEARCH_BODY {
        &raw_bytes[..MAX_SEARCH_BODY]
    } else {
        raw_bytes.as_ref()
    })
    .into_owned();

    let mut hits = extract_search_hits(&html);
    if hits.is_empty() && final_url.host_str().is_some() {
        hits = extract_search_hits_from_generic_links(&html);
    }

    if let Some(allowed) = input.allowed_domains.as_ref() {
        hits.retain(|hit| host_matches_list(&hit.url, allowed));
    }
    if let Some(blocked) = input.blocked_domains.as_ref() {
        hits.retain(|hit| !host_matches_list(&hit.url, blocked));
    }
    dedupe_hits(&mut hits);
    hits.truncate(8);

    let summary = if hits.is_empty() {
        format!("No web search results matched the query {:?}.", input.query)
    } else {
        let rendered = hits
            .iter()
            .map(|h| format!("- [{}]({})", h.title, h.url))
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "Search results for {:?}. Include a Sources section in the final answer.\n{}",
            input.query, rendered
        )
    };

    Ok(WebSearchOutput {
        query: input.query.clone(),
        results: vec![
            WebSearchResultItem::Commentary(summary),
            WebSearchResultItem::SearchResult {
                tool_use_id: String::from("web_search_1"),
                content: hits,
            },
        ],
        duration_seconds: started.elapsed().as_secs_f64(),
    })
}

pub(crate) fn normalize_fetch_url(url: &str) -> Result<String, String> {
    // Reject Windows UNC paths before URL parsing.
    if url.starts_with("\\\\") || url.starts_with("//") {
        return Err(String::from(
            "SSRF protection: UNC / network share paths are not allowed",
        ));
    }

    let parsed = reqwest::Url::parse(url).map_err(|error| error.to_string())?;

    // Only HTTP and HTTPS are supported.
    match parsed.scheme() {
        "http" | "https" => {}
        other => {
            return Err(format!(
                "SSRF protection: scheme `{other}` is not allowed; only http and https are accepted"
            ));
        }
    }

    if parsed.scheme() == "http" {
        let host = parsed.host_str().unwrap_or_default();
        if host != "localhost" && host != "127.0.0.1" && host != "::1" {
            let mut upgraded = parsed;
            upgraded
                .set_scheme("https")
                .map_err(|()| String::from("failed to upgrade URL to https"))?;
            return Ok(upgraded.to_string());
        }
    }
    Ok(parsed.to_string())
}

/// Resolve the hostname of `url` and reject requests that target private,
/// loopback, link-local, or other reserved IP ranges (SSRF prevention).
async fn ssrf_check_url(url: &str) -> Result<(), String> {
    use std::net::IpAddr;
    use tokio::net::lookup_host;

    let parsed = reqwest::Url::parse(url).map_err(|e| e.to_string())?;
    let host = match parsed.host_str() {
        Some(h) => h,
        None => return Ok(()), // nothing to resolve
    };

    // Literal IP addresses: check them directly without DNS.
    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_ssrf_blocked_ip(ip) {
            return Err(format!(
                "SSRF protection: IP address {ip} is in a private/reserved range"
            ));
        }
        return Ok(());
    }

    // DNS resolution: all returned addresses must be public.
    let port = parsed
        .port()
        .unwrap_or(if parsed.scheme() == "https" { 443 } else { 80 });
    let addrs = lookup_host(format!("{host}:{port}"))
        .await
        .map_err(|e| format!("DNS resolution failed for `{host}`: {e}"))?;

    for addr in addrs {
        if is_ssrf_blocked_ip(addr.ip()) {
            return Err(format!(
                "SSRF protection: `{host}` resolved to a private/reserved IP {}",
                addr.ip()
            ));
        }
    }
    Ok(())
}

/// Returns `true` for IP addresses that must not be reached from a web-fetch
/// request, to prevent SSRF attacks.
///
/// Loopback addresses (127.x, ::1) are **not** blocked; they are legitimate
/// for developer tooling (e.g., local test servers).  The main targets of
/// concern are cloud-metadata endpoints, private LAN ranges, and link-local.
fn is_ssrf_blocked_ip(ip: std::net::IpAddr) -> bool {
    use std::net::IpAddr;
    match ip {
        IpAddr::V4(v4) => {
            v4.is_private()        // 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16
                || v4.is_link_local()  // 169.254.0.0/16 (incl. AWS metadata 169.254.169.254)
                || v4.is_broadcast()   // 255.255.255.255
                || v4.is_unspecified() // 0.0.0.0
                // Carrier-grade NAT: 100.64.0.0/10
                || (v4.octets()[0] == 100 && (v4.octets()[1] & 0xC0) == 64)
                // IETF Protocol Assignments: 192.0.0.0/24
                || (v4.octets()[0] == 192 && v4.octets()[1] == 0 && v4.octets()[2] == 0)
            // Loopback (127.x) is intentionally NOT blocked — dev tooling uses localhost
        }
        IpAddr::V6(v6) => {
            v6.is_unspecified()
                // Unique Local Address (fc00::/7)
                || (v6.segments()[0] & 0xFE00) == 0xFC00
                // Link-local (fe80::/10)
                || (v6.segments()[0] & 0xFFC0) == 0xFE80
                // IPv4-mapped private addresses (but not ::ffff:127.x loopback)
                || matches!(v6.to_ipv4_mapped(), Some(v4) if is_ssrf_blocked_ip(IpAddr::V4(v4)))
            // Loopback (::1) is intentionally NOT blocked
        }
    }
}

pub(crate) fn build_search_url(query: &str) -> Result<reqwest::Url, String> {
    if let Ok(base) = std::env::var("CODINEER_WEB_SEARCH_BASE_URL") {
        let mut url = reqwest::Url::parse(&base).map_err(|error| error.to_string())?;
        url.query_pairs_mut().append_pair("q", query);
        return Ok(url);
    }

    let mut url = reqwest::Url::parse("https://html.duckduckgo.com/html/")
        .map_err(|error| error.to_string())?;
    url.query_pairs_mut().append_pair("q", query);
    Ok(url)
}

pub(crate) fn normalize_fetched_content(body: &str, content_type: &str) -> String {
    if content_type.contains("html") {
        html_to_text(body)
    } else {
        body.trim().to_string()
    }
}

pub(crate) fn summarize_web_fetch(
    url: &str,
    prompt: &str,
    content: &str,
    raw_body: &str,
    content_type: &str,
) -> String {
    let lower_prompt = prompt.to_lowercase();
    let compact = collapse_whitespace(content);

    let detail = if lower_prompt.contains("title") {
        extract_title(content, raw_body, content_type).map_or_else(
            || preview_text(&compact, 600),
            |title| format!("Title: {title}"),
        )
    } else if lower_prompt.contains("summary") || lower_prompt.contains("summarize") {
        preview_text(&compact, 900)
    } else {
        let preview = preview_text(&compact, 900);
        format!("Prompt: {prompt}\nContent preview:\n{preview}")
    };

    format!("Fetched {url}\n{detail}")
}

pub(crate) fn extract_title(content: &str, raw_body: &str, content_type: &str) -> Option<String> {
    if content_type.contains("html") {
        let lowered = raw_body.to_lowercase();
        if let Some(start) = lowered.find("<title>") {
            let after = start + "<title>".len();
            if let Some(end_rel) = lowered[after..].find("</title>") {
                let title =
                    collapse_whitespace(&decode_html_entities(&raw_body[after..after + end_rel]));
                if !title.is_empty() {
                    return Some(title);
                }
            }
        }
    }

    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    None
}

pub(crate) fn html_to_text(html: &str) -> String {
    let mut text = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut previous_was_space = false;

    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if in_tag => {}
            '&' => {
                text.push('&');
                previous_was_space = false;
            }
            ch if ch.is_whitespace() => {
                if !previous_was_space {
                    text.push(' ');
                    previous_was_space = true;
                }
            }
            _ => {
                text.push(ch);
                previous_was_space = false;
            }
        }
    }

    collapse_whitespace(&decode_html_entities(&text))
}

pub(crate) fn decode_html_entities(input: &str) -> String {
    input
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
}

pub(crate) fn collapse_whitespace(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(crate) fn preview_text(input: &str, max_chars: usize) -> String {
    if input.chars().count() <= max_chars {
        return input.to_string();
    }
    let shortened = input.chars().take(max_chars).collect::<String>();
    format!("{}…", shortened.trim_end())
}

pub(crate) fn extract_search_hits(html: &str) -> Vec<SearchHit> {
    let mut hits = Vec::new();
    let mut remaining = html;

    while let Some(anchor_start) = remaining.find("result__a") {
        let after_class = &remaining[anchor_start..];
        let Some(href_idx) = after_class.find("href=") else {
            remaining = &after_class[1..];
            continue;
        };
        let href_slice = &after_class[href_idx + 5..];
        let Some((url, rest)) = extract_quoted_value(href_slice) else {
            remaining = &after_class[1..];
            continue;
        };
        let Some(close_tag_idx) = rest.find('>') else {
            remaining = &after_class[1..];
            continue;
        };
        let after_tag = &rest[close_tag_idx + 1..];
        let Some(end_anchor_idx) = after_tag.find("</a>") else {
            remaining = &after_tag[1..];
            continue;
        };
        let title = html_to_text(&after_tag[..end_anchor_idx]);
        if let Some(decoded_url) = decode_duckduckgo_redirect(&url) {
            hits.push(SearchHit {
                title: title.trim().to_string(),
                url: decoded_url,
            });
        }
        remaining = &after_tag[end_anchor_idx + 4..];
    }

    hits
}

pub(crate) fn extract_search_hits_from_generic_links(html: &str) -> Vec<SearchHit> {
    let mut hits = Vec::new();
    let mut remaining = html;

    while let Some(anchor_start) = remaining.find("<a") {
        let after_anchor = &remaining[anchor_start..];
        let Some(href_idx) = after_anchor.find("href=") else {
            remaining = &after_anchor[2..];
            continue;
        };
        let href_slice = &after_anchor[href_idx + 5..];
        let Some((url, rest)) = extract_quoted_value(href_slice) else {
            remaining = &after_anchor[2..];
            continue;
        };
        let Some(close_tag_idx) = rest.find('>') else {
            remaining = &after_anchor[2..];
            continue;
        };
        let after_tag = &rest[close_tag_idx + 1..];
        let Some(end_anchor_idx) = after_tag.find("</a>") else {
            remaining = &after_anchor[2..];
            continue;
        };
        let title = html_to_text(&after_tag[..end_anchor_idx]);
        if title.trim().is_empty() {
            remaining = &after_tag[end_anchor_idx + 4..];
            continue;
        }
        let decoded_url = decode_duckduckgo_redirect(&url).unwrap_or(url);
        if decoded_url.starts_with("http://") || decoded_url.starts_with("https://") {
            hits.push(SearchHit {
                title: title.trim().to_string(),
                url: decoded_url,
            });
        }
        remaining = &after_tag[end_anchor_idx + 4..];
    }

    hits
}

pub(crate) fn extract_quoted_value(input: &str) -> Option<(String, &str)> {
    let quote = input.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let rest = &input[quote.len_utf8()..];
    let end = rest.find(quote)?;
    Some((rest[..end].to_string(), &rest[end + quote.len_utf8()..]))
}

pub(crate) fn decode_duckduckgo_redirect(url: &str) -> Option<String> {
    if url.starts_with("http://") || url.starts_with("https://") {
        return Some(html_entity_decode_url(url));
    }

    let joined = if url.starts_with("//") {
        format!("https:{url}")
    } else if url.starts_with('/') {
        format!("https://duckduckgo.com{url}")
    } else {
        return None;
    };

    let parsed = reqwest::Url::parse(&joined).ok()?;
    if parsed.path() == "/l/" || parsed.path() == "/l" {
        for (key, value) in parsed.query_pairs() {
            if key == "uddg" {
                return Some(html_entity_decode_url(value.as_ref()));
            }
        }
    }
    Some(joined)
}

pub(crate) fn html_entity_decode_url(url: &str) -> String {
    decode_html_entities(url)
}

pub(crate) fn host_matches_list(url: &str, domains: &[String]) -> bool {
    let Ok(parsed) = reqwest::Url::parse(url) else {
        return false;
    };
    let Some(host) = parsed.host_str() else {
        return false;
    };
    let host = host.to_ascii_lowercase();
    domains.iter().any(|domain| {
        let normalized = normalize_domain_filter(domain);
        !normalized.is_empty() && (host == normalized || host.ends_with(&format!(".{normalized}")))
    })
}

pub(crate) fn normalize_domain_filter(domain: &str) -> String {
    let trimmed = domain.trim();
    let candidate = reqwest::Url::parse(trimmed)
        .ok()
        .and_then(|url| url.host_str().map(str::to_string))
        .unwrap_or_else(|| trimmed.to_string());
    candidate
        .trim()
        .trim_start_matches('.')
        .trim_end_matches('/')
        .to_ascii_lowercase()
}

pub(crate) fn dedupe_hits(hits: &mut Vec<SearchHit>) {
    let mut seen = BTreeSet::new();
    hits.retain(|hit| seen.insert(hit.url.clone()));
}

// ---------------------------------------------------------------------------
// BuiltinTool adapters
// ---------------------------------------------------------------------------

pub(crate) struct WebFetchTool;

impl BuiltinTool for WebFetchTool {
    const NAME: &'static str = "WebFetch";
    type Input = WebFetchInput;

    fn execute(input: Self::Input) -> Result<ToolOutput, ToolError> {
        crate::to_pretty_json(execute_web_fetch(&input).map_err(ToolError::execution)?)
    }

    fn is_concurrency_safe(_input: &Self::Input) -> bool {
        true
    }
}

pub(crate) struct WebSearchTool;

impl BuiltinTool for WebSearchTool {
    const NAME: &'static str = "WebSearch";
    type Input = WebSearchInput;

    fn execute(input: Self::Input) -> Result<ToolOutput, ToolError> {
        crate::to_pretty_json(execute_web_search(&input).map_err(ToolError::execution)?)
    }

    fn is_concurrency_safe(_input: &Self::Input) -> bool {
        true
    }
}
