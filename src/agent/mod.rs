use crate::model::SearchResponse;
use serde::{Deserialize, Serialize};

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const MAX_TOKENS: u32 = 8192;
// Gemini's thinking-enabled models count reasoning tokens against maxOutputTokens,
// so the visible JSON response can get truncated well before 8192 tokens of output.
const GEMINI_MAX_OUTPUT_TOKENS: u32 = 32768;

pub async fn run(
    provider: &str,
    api_key: &str,
    model: &str,
    category: Option<&str>,
    query: &str,
    refinement: Option<&str>,
    search_results: &str,
) -> (Result<SearchResponse, String>, String, String) {
    if search_results.trim().is_empty() {
        let msg = "No search results to analyze.".to_string();
        return (Err(msg.clone()), String::new(), msg);
    }

    let prompt = build_prompt(category, query, refinement, search_results);
    let result_count = search_results.lines().filter(|l| l.starts_with('[') ).count();
    let req_summary = format!(
        "Provider: {}\nModel: {}\nSearch results injected: {}\n\nPrompt:\n{}",
        provider,
        model,
        result_count,
        &prompt
    );

    let raw = match provider {
        "gemini" => call_gemini(api_key, model, &prompt).await,
        "chatgpt" => call_chatgpt(api_key, model, &prompt).await,
        _ => call_claude(api_key, model, &prompt).await,
    };

    match raw {
        Err(e) => (Err(e.clone()), req_summary, e),
        Ok(raw) => {
            let result = parse_response(&raw);
            (result, req_summary, raw)
        }
    }
}

fn build_prompt(
    category: Option<&str>,
    query: &str,
    refinement: Option<&str>,
    search_results: &str,
) -> String {
    let mut prompt = String::from("You are a personal shopping assistant");
    if let Some(cat) = category {
        prompt.push_str(&format!(", an expert in the {} category", cat));
    }
    prompt.push_str(".\nThe user is searching for:\n");
    prompt.push_str(query);
    if let Some(r) = refinement {
        prompt.push_str(&format!(
            "\nAdditional instructions to refine results: {}",
            r
        ));
    }
    prompt.push_str(r#"

Use the search results provided below, analyze them and group by brand and model (different sellers offering the same product are one group), then return a JSON object matching this structure exactly:

{
  "groups": [
    {
      "brand": "Brand name",
      "model": "Model name",
      "image": null,
      "match_score": 0,
      "listings": [
        {
          "source": "Store name",
          "price": "Price with currency",
          "url": "Link",
          "delivery": "Free delivery",
          "in_stock": true
        }
      ]
    }
  ]
}

Grouping rules:
- One group per distinct brand+model combination. Different sellers offering the same product must be in one group.
- Treat product titles as the same model if they refer to the same physical item despite wording differences. Examples: "LS200" and "Lexus LS200" are the same; "Nissan Silvia S15" and "S15 200mm body" are the same; abbreviated names, part numbers, and full names for the same product must be merged.
- Normalise the brand field: use the manufacturer name only (e.g. "HPI Racing" → brand "HPI Racing", model "Nissan Silvia S15 200mm Body Shell").

Scoring rules:
- match_score (0-100) reflects how closely this product matches the user's query specs. Be strict.
- Penalise heavily (score ≤ 30) any result that contradicts a numeric spec in the query. If the user asked for 200mm, a 190mm body scores ≤ 30. If the user asked for 1/10 scale, a 1/8 scale body scores ≤ 20.
- Score 80–100 only if the product matches all key specs (scale, width, type).
- Sort groups by match_score descending.

Output rules:
- Within each group, sort listings by price ascending (lowest price first).
- If stock status cannot be determined, set in_stock to null (not false).
- The delivery field is optional — omit if not available.
- Always set image to null. Images are handled separately.
- Return only the JSON object. No explanation, no markdown fencing.

Search results:
"#);
    prompt.push_str(search_results);
    prompt
}

#[derive(Serialize)]
struct ClaudeRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    messages: Vec<ClaudeMessage<'a>>,
}

#[derive(Serialize)]
struct ClaudeMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContent>,
    error: Option<ClaudeError>,
}

#[derive(Deserialize)]
struct ClaudeContent {
    text: Option<String>,
}

#[derive(Deserialize)]
struct ClaudeError {
    message: String,
}

async fn call_claude(api_key: &str, model: &str, prompt: &str) -> Result<String, String> {
    let client = reqwest::Client::new();

    let request_body = ClaudeRequest {
        model,
        max_tokens: MAX_TOKENS,
        messages: vec![ClaudeMessage {
            role: "user",
            content: prompt,
        }],
    };

    let response = client
        .post(ANTHROPIC_API_URL)
        .header("x-api-key", api_key)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .header("content-type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Claude request failed: {}", e))?;

    let status = response.status();
    let raw = response
        .text()
        .await
        .map_err(|e| format!("Failed to read Claude response: {}", e))?;

    if status == 401 {
        return Err(format!("Invalid Anthropic API key: {}", raw));
    }
    if status.as_u16() == 429 {
        return Err(format!("Anthropic rate limit exceeded: {}", raw));
    }
    if !status.is_success() {
        return Err(format!("Claude API error {}: {}", status, raw));
    }

    let body: ClaudeResponse = serde_json::from_str(&raw)
        .map_err(|e| format!("Failed to parse Claude response: {}", e))?;

    if let Some(err) = body.error {
        return Err(format!("Claude error: {}", err.message));
    }

    body.content
        .into_iter()
        .find_map(|c| c.text)
        .ok_or_else(|| "Empty response from Claude.".to_string())
}

// ── Gemini ──────────────────────────────────────────────────────────────────

const GEMINI_API_URL: &str =
    "https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent";

async fn call_gemini(api_key: &str, model: &str, prompt: &str) -> Result<String, String> {
    let url = format!(
        "{}?key={}",
        GEMINI_API_URL.replace("{model}", model),
        api_key
    );

    let body = serde_json::json!({
        "contents": [{ "parts": [{ "text": prompt }] }],
        "generationConfig": { "maxOutputTokens": GEMINI_MAX_OUTPUT_TOKENS }
    });

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Gemini request failed: {}", e))?;

    let status = response.status();
    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Gemini response: {}", e))?;

    if status == 400 {
        return Err(format!("Invalid Gemini API key or model: {}", body));
    }
    if status.as_u16() == 429 {
        return Err(format!("Gemini rate limit exceeded: {}", body));
    }
    if !status.is_success() {
        return Err(format!("Gemini API error {}: {}", status, body));
    }

    body["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "Empty response from Gemini.".to_string())
}

// ── ChatGPT ──────────────────────────────────────────────────────────────────

const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";

async fn call_chatgpt(api_key: &str, model: &str, prompt: &str) -> Result<String, String> {
    let body = serde_json::json!({
        "model": model,
        "messages": [{ "role": "user", "content": prompt }],
        "max_tokens": MAX_TOKENS
    });

    let client = reqwest::Client::new();
    let response = client
        .post(OPENAI_API_URL)
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("ChatGPT request failed: {}", e))?;

    let status = response.status();
    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse ChatGPT response: {}", e))?;

    if status == 401 {
        return Err(format!("Invalid OpenAI API key: {}", body));
    }
    if status.as_u16() == 429 {
        return Err(format!("OpenAI rate limit exceeded: {}", body));
    }
    if !status.is_success() {
        return Err(format!("ChatGPT API error {}: {}", status, body));
    }

    body["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "Empty response from ChatGPT.".to_string())
}

fn parse_response(raw: &str) -> Result<SearchResponse, String> {
    // Strip markdown fencing if Claude adds it despite instructions
    let clean = raw.trim();
    let clean = clean
        .strip_prefix("```json")
        .or_else(|| clean.strip_prefix("```"))
        .unwrap_or(clean);
    let clean = clean.strip_suffix("```").unwrap_or(clean).trim();

    serde_json::from_str(clean)
        .map_err(|e| format!("Failed to parse results: {}\n\n--- Last 300 chars of response ---\n{}", e, &clean[clean.len().saturating_sub(300)..]))
}
