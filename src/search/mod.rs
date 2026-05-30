// Value SERP shopping search client
// Docs: https://www.valueserp.com/docs/search-api/overview

use serde::{Deserialize, Deserializer};

fn deserialize_string_or_number<'de, D>(d: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let v: Option<serde_json::Value> = Option::deserialize(d)?;
    Ok(v.map(|val| match val {
        serde_json::Value::String(s) => s,
        other => other.to_string(),
    }))
}

pub struct SearchResult {
    pub title: String,
    pub merchant: String,
    pub url: String,
    pub price: String,
    pub image: Option<String>,
    pub delivery: Option<String>,
}

#[derive(Deserialize)]
struct ValueSerpResponse {
    shopping_results: Option<Vec<ValueSerpItem>>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct ValueSerpItem {
    title: Option<String>,
    merchant: Option<String>,
    link: Option<String>,
    id: Option<String>,
    #[serde(default, deserialize_with = "deserialize_string_or_number")]
    price: Option<String>,
    image: Option<String>,
    #[serde(default, deserialize_with = "deserialize_string_or_number")]
    delivery: Option<String>,
}

/// Build the SERP query string, appending `-term` for each exclude token.
pub fn build_serp_query(query: &str, exclude: Option<&str>) -> String {
    let mut q = query.to_string();
    if let Some(ex) = exclude {
        for token in ex.split([',', ' ']) {
            let token = token.trim();
            if !token.is_empty() {
                q.push(' ');
                q.push('-');
                q.push_str(token);
            }
        }
    }
    q
}

pub async fn search(
    api_key: &str,
    query: &str,
    exclude: Option<&str>,
    location: Option<&str>,
    pages: u32,
) -> (Result<Vec<SearchResult>, String>, String, String) {
    let serp_query = build_serp_query(query, exclude);
    let pages = pages.clamp(1, 5);

    let req_summary = format!(
        "GET valueserp q=\"{}\" location={} pages={}",
        serp_query,
        location.unwrap_or("(none)"),
        pages
    );

    // Fire all page requests concurrently
    let client = reqwest::Client::new();
    let futures: Vec<_> = (1..=pages)
        .map(|page| {
            let client = client.clone();
            let api_key = api_key.to_string();
            let serp_query = serp_query.clone();
            let location = location.map(|s| s.to_string());
            async move {
                let mut url = format!(
                    "https://api.valueserp.com/search?api_key={}&search_type=shopping&num=40&q={}",
                    api_key,
                    urlencoding::encode(&serp_query),
                );
                if page > 1 {
                    url.push_str(&format!("&page={}", page));
                }
                if let Some(loc) = &location {
                    url.push_str(&format!("&location={}", urlencoding::encode(loc)));
                }
                client.get(&url).send().await
            }
        })
        .collect();

    let responses = futures::future::join_all(futures).await;

    let mut all_results: Vec<SearchResult> = Vec::new();
    let mut raw_pages: Vec<String> = Vec::new();
    let mut first_error: Option<String> = None;

    for (i, resp) in responses.into_iter().enumerate() {
        let page_num = i + 1;
        let response = match resp {
            Ok(r) => r,
            Err(e) => {
                if first_error.is_none() {
                    first_error = Some(format!("Search request failed (page {}): {}", page_num, e));
                }
                continue;
            }
        };

        let status = response.status();

        if status == 401 || status == 403 {
            return (
                Err("Invalid Value SERP API key.".to_string()),
                req_summary,
                format!("HTTP {}", status),
            );
        }
        if status.as_u16() == 429 {
            return (
                Err("Value SERP daily quota exceeded (100 requests/day on free tier).".to_string()),
                req_summary,
                format!("HTTP {}", status),
            );
        }

        let raw = match response.text().await {
            Ok(t) => t,
            Err(e) => {
                if first_error.is_none() {
                    first_error = Some(format!("Failed to read response (page {}): {}", page_num, e));
                }
                continue;
            }
        };

        let pretty = serde_json::from_str::<serde_json::Value>(&raw)
            .map(|v| serde_json::to_string_pretty(&v).unwrap_or(raw.clone()))
            .unwrap_or(raw.clone());
        raw_pages.push(format!("=== page {} ===\n{}", page_num, pretty));

        if !status.is_success() {
            if first_error.is_none() {
                first_error = Some(format!("Search API error page {}: {}", page_num, status));
            }
            continue;
        }

        let body: ValueSerpResponse = match serde_json::from_str(&raw) {
            Ok(b) => b,
            Err(e) => {
                if first_error.is_none() {
                    first_error = Some(format!("Failed to parse response (page {}): {}", page_num, e));
                }
                continue;
            }
        };

        if let Some(err) = body.error {
            return (
                Err(format!("Value SERP error: {}", err)),
                req_summary,
                raw_pages.join("\n\n"),
            );
        }

        let page_results: Vec<SearchResult> = body
            .shopping_results
            .unwrap_or_default()
            .into_iter()
            .filter_map(|item| {
                let title = item.title?;
                let url = item.link
                    .filter(|l| !l.is_empty())
                    .or_else(|| item.id.as_deref().map(|id| {
                        format!("https://www.google.com/shopping/product/{}", id)
                    }))
                    .unwrap_or_else(|| {
                        format!("https://www.google.com/search?q={}", urlencoding::encode(&title))
                    });
                Some(SearchResult {
                    title,
                    merchant: item.merchant.unwrap_or_else(|| "Unknown".to_string()),
                    url,
                    price: item.price.unwrap_or_else(|| "—".to_string()),
                    image: item.image,
                    delivery: item.delivery,
                })
            })
            .collect();

        all_results.extend(page_results);
    }

    let combined_raw = raw_pages.join("\n\n");

    if all_results.is_empty() {
        if let Some(e) = first_error {
            return (Err(e), req_summary, combined_raw);
        }
        return (Err("No results returned.".to_string()), req_summary, combined_raw);
    }

    let req_summary = format!(
        "{}\n\n--- parsed: {} results, {} with images ---",
        req_summary,
        all_results.len(),
        all_results.iter().filter(|r| r.image.is_some()).count()
    );

    (Ok(all_results), req_summary, combined_raw)
}

pub fn format_for_prompt(results: &[SearchResult]) -> String {
    results
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let mut entry = format!(
                "[{}] title: {}\n    store: {}\n    url: {}\n    price: {}",
                i + 1,
                r.title,
                r.merchant,
                r.url,
                r.price,
            );
            if let Some(delivery) = &r.delivery {
                entry.push_str(&format!("\n    delivery: {}", delivery));
            }
            entry
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── build_serp_query ────────────────────────────────────────────────────

    #[test]
    fn build_serp_query_no_excludes() {
        assert_eq!(build_serp_query("drone fpv", None), "drone fpv");
    }

    #[test]
    fn build_serp_query_space_separated_excludes() {
        let q = build_serp_query("drone fpv", Some("cheap used"));
        assert!(q.contains("-cheap"), "expected -cheap in query");
        assert!(q.contains("-used"), "expected -used in query");
    }

    #[test]
    fn build_serp_query_comma_separated_excludes() {
        let q = build_serp_query("drone fpv", Some("cheap,used"));
        assert!(q.contains("-cheap"));
        assert!(q.contains("-used"));
    }

    #[test]
    fn build_serp_query_empty_exclude_string() {
        // An empty exclude string should produce no -tokens
        let q = build_serp_query("drone fpv", Some(""));
        assert_eq!(q, "drone fpv");
    }

    #[test]
    fn build_serp_query_whitespace_only_tokens_skipped() {
        let q = build_serp_query("drone fpv", Some("  ,  "));
        assert_eq!(q, "drone fpv");
    }

    // ── format_for_prompt ──────────────────────────────────────────────────

    fn make_result(title: &str, merchant: &str, price: &str, delivery: Option<&str>) -> SearchResult {
        SearchResult {
            title: title.to_string(),
            merchant: merchant.to_string(),
            url: format!("https://example.com/{}", title),
            price: price.to_string(),
            image: None,
            delivery: delivery.map(|s| s.to_string()),
        }
    }

    #[test]
    fn format_for_prompt_empty_slice() {
        assert_eq!(format_for_prompt(&[]), "");
    }

    #[test]
    fn format_for_prompt_single_result_contains_fields() {
        let results = vec![make_result("Avata 2", "Amazon", "$499", None)];
        let out = format_for_prompt(&results);
        assert!(out.contains("[1]"));
        assert!(out.contains("Avata 2"));
        assert!(out.contains("Amazon"));
        assert!(out.contains("$499"));
        assert!(!out.contains("delivery:"), "no delivery line when None");
    }

    #[test]
    fn format_for_prompt_delivery_line_present_when_some() {
        let results = vec![make_result("Avata 2", "Amazon", "$499", Some("Free shipping"))];
        let out = format_for_prompt(&results);
        assert!(out.contains("delivery: Free shipping"));
    }

    #[test]
    fn format_for_prompt_indices_are_sequential() {
        let results = vec![
            make_result("A", "S1", "$1", None),
            make_result("B", "S2", "$2", None),
            make_result("C", "S3", "$3", None),
        ];
        let out = format_for_prompt(&results);
        assert!(out.contains("[1]"));
        assert!(out.contains("[2]"));
        assert!(out.contains("[3]"));
    }
}
