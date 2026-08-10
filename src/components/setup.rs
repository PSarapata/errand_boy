use crate::components::provider_picker::{Provider, ProviderPicker};
use crate::config;
use crate::model::AppConfig;
use crate::LOGO1;
use dioxus::prelude::*;

#[derive(Debug, Clone, PartialEq)]
enum SetupStep {
    PickProvider,
    EnterKey(Provider),
    PickModel(Provider, Vec<String>),
    SearchKey(Provider, String),
}

#[component]
pub fn Setup(
    existing: Option<AppConfig>,
    on_done: EventHandler<AppConfig>,
    on_cancel: EventHandler<()>,
) -> Element {
    // Editing saved credentials: prefill every field and skip the provider picker, so
    // opening settings by accident can never wipe the stored keys.
    let editing = existing.is_some();
    let saved_provider = existing
        .as_ref()
        .and_then(|c| Provider::from_id(&c.llm_provider));
    let (saved_llm_key, saved_model, saved_search_key) = match &existing {
        Some(c) => (
            c.llm_api_key.clone(),
            c.llm_model.clone(),
            c.search_api_key.clone(),
        ),
        None => (String::new(), String::new(), String::new()),
    };

    let initial_step = match saved_provider {
        Some(p) => SetupStep::EnterKey(p),
        None => SetupStep::PickProvider,
    };
    let mut step = use_signal(|| initial_step);
    let mut llm_key = use_signal(|| saved_llm_key);
    let mut selected_model = use_signal(|| saved_model);
    let mut search_key = use_signal(|| saved_search_key);
    let mut error = use_signal(String::new);
    let mut loading = use_signal(|| false);

    let body = match step() {
        SetupStep::PickProvider => rsx! {
            ProviderPicker {
                on_pick: move |p| {
                    llm_key.set(String::new());
                    selected_model.set(String::new());
                    error.set(String::new());
                    step.set(SetupStep::EnterKey(p));
                }
            }
        },

        SetupStep::EnterKey(provider) => {
            let placeholder = provider.key_placeholder();
            let provider_label = provider.label();
            let api_key_url = provider.api_key_url();
            rsx! {
                div { class: "setup-screen",
                    img { src: LOGO1, class: "setup-logo", alt: "Errand Boy" }
                    h2 { "{provider_label} API key" }
                    p {
                        if editing {
                            "Your saved {provider_label} key is filled in — leave it as is to keep it. "
                        } else {
                            "Enter your {provider_label} API key. "
                        }
                        a { href: "{api_key_url}", target: "_blank", "Get one here →" }
                    }
                    input {
                        r#type: "password",
                        placeholder: "{placeholder}",
                        value: "{llm_key}",
                        oninput: move |e| {
                            llm_key.set(e.value());
                            error.set(String::new());
                        }
                    }
                    if !error().is_empty() {
                        p { class: "error", "{error}" }
                    }
                    div { class: "setup-row",
                        button {
                            class: "btn-secondary",
                            onclick: move |_| {
                                error.set(String::new());
                                step.set(SetupStep::PickProvider);
                            },
                            "← Change provider"
                        }
                        button {
                            disabled: loading() || llm_key().trim().is_empty(),
                            onclick: {
                                let provider = provider.clone();
                                move |_| {
                                    let key = llm_key().trim().to_string();
                                    let provider = provider.clone();
                                    async move {
                                        loading.set(true);
                                        error.set(String::new());
                                        match fetch_models(&provider, &key).await {
                                            Ok(models) => step.set(SetupStep::PickModel(provider, models)),
                                            Err(e) => error.set(e),
                                        }
                                        loading.set(false);
                                    }
                                }
                            },
                            if loading() { "Checking key…" } else { "Continue" }
                        }
                    }
                }
            }
        },

        SetupStep::PickModel(provider, models) => {
            // Keep a saved model when the account still offers it; otherwise fall back.
            if !models.contains(&selected_model())
                && let Some(default) = models.first() {
                    selected_model.set(default.clone());
                }
            let provider_label = provider.label();
            rsx! {
                div { class: "setup-screen",
                    img { src: LOGO1, class: "setup-logo", alt: "Errand Boy" }
                    h2 { "Choose a {provider_label} model" }
                    select {
                        value: "{selected_model}",
                        onchange: move |e| selected_model.set(e.value()),
                        for model in models.iter() {
                            option { value: "{model}", "{model}" }
                        }
                    }
                    div { class: "setup-row",
                        button {
                            class: "btn-secondary",
                            onclick: {
                                let provider = provider.clone();
                                move |_| step.set(SetupStep::EnterKey(provider.clone()))
                            },
                            "← Back"
                        }
                        button {
                            disabled: selected_model().is_empty(),
                            onclick: {
                                let provider = provider.clone();
                                move |_| step.set(SetupStep::SearchKey(provider.clone(), selected_model()))
                            },
                            "Continue"
                        }
                    }
                }
            }
        },

        SetupStep::SearchKey(provider, model) => rsx! {
            div { class: "setup-screen",
                img { src: LOGO1, class: "setup-logo", alt: "Errand Boy" }
                h2 { "Value SERP API key" }
                p {
                    if editing {
                        "Your saved Value SERP key is filled in — leave it as is to keep it. "
                    } else {
                        "Enter your Value SERP key for product search. "
                    }
                    a { href: "https://app.valueserp.com/signup", target: "_blank", "Get one free." }
                }
                input {
                    r#type: "password",
                    placeholder: "Value SERP API key",
                    value: "{search_key}",
                    oninput: move |e| {
                        search_key.set(e.value());
                        error.set(String::new());
                    }
                }
                if !error().is_empty() {
                    p { class: "error", "{error}" }
                }
                div { class: "setup-row",
                    button {
                        class: "btn-secondary",
                        onclick: {
                            let provider = provider.clone();
                            move |_| {
                                // Back to the LLM key step — Continue there re-fetches the full
                                // model list, which going straight to PickModel could not do.
                                error.set(String::new());
                                step.set(SetupStep::EnterKey(provider.clone()));
                            }
                        },
                        "← Back"
                    }
                    button {
                        disabled: search_key().trim().is_empty(),
                        onclick: {
                            let provider = provider.clone();
                            let model = model.clone();
                            move |_| {
                                let config = AppConfig {
                                    llm_provider: provider.id().to_string(),
                                    llm_model: model.clone(),
                                    llm_api_key: llm_key().trim().to_string(),
                                    search_api_key: search_key().trim().to_string(),
                                };
                                if let Err(e) = config::save(&config) {
                                    error.set(format!("Failed to save config: {}", e));
                                } else {
                                    on_done.call(config);
                                }
                            }
                        },
                        if editing { "Save" } else { "Finish setup" }
                    }
                }
            }
        },
    };

    rsx! {
        // Escape hatch: only when there is a config to come back to.
        if editing {
            button {
                class: "setup-escape",
                onclick: move |_| on_cancel.call(()),
                "✕ Back to the app"
            }
        }
        {body}
    }
}

async fn fetch_models(provider: &Provider, api_key: &str) -> Result<Vec<String>, String> {
    let client = reqwest::Client::new();

    let response = match provider {
        Provider::Gemini => {
            client
                .get(format!("{}?key={}", provider.models_url(), api_key))
                .send()
                .await
        }
        Provider::Claude => {
            client
                .get(provider.models_url())
                .header("x-api-key", api_key)
                .header("anthropic-version", "2023-06-01")
                .send()
                .await
        }
        Provider::ChatGpt => {
            client
                .get(provider.models_url())
                .bearer_auth(api_key)
                .send()
                .await
        }
    }
    .map_err(|e| format!("Network error: {}", e))?;

    if response.status() == 401 || response.status() == 403 {
        return Err("Invalid API key.".to_string());
    }
    if !response.status().is_success() {
        return Err(format!("API error: {}", response.status()));
    }

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let models: Vec<String> = match provider {
        Provider::Gemini => body["models"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|m| {
                let name = m["name"].as_str()?;
                // Only include generateContent-capable models
                let supported: Vec<&str> = m["supportedGenerationMethods"]
                    .as_array()?
                    .iter()
                    .filter_map(|v| v.as_str())
                    .collect();
                if supported.contains(&"generateContent") {
                    // Strip "models/" prefix
                    Some(name.trim_start_matches("models/").to_string())
                } else {
                    None
                }
            })
            .collect(),
        Provider::Claude => body["data"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|m| m["id"].as_str().map(|s| s.to_string()))
            .collect(),
        Provider::ChatGpt => body["data"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|m| m["id"].as_str().map(|s| s.to_string()))
            .filter(|id| id.starts_with("gpt"))
            .collect(),
    };

    if models.is_empty() {
        return Err("No compatible models found for this API key.".to_string());
    }

    let mut sorted = models;
    match provider {
        Provider::Gemini => {
            sorted.sort_by_key(|m| {
                if m.contains("pro") && m.contains("exp") { 0 }
                else if m.contains("2.0") { 1 }
                else if m.contains("1.5-pro") { 2 }
                else if m.contains("1.5-flash") { 3 }
                else { 4 }
            });
        }
        Provider::Claude => {
            sorted.sort_by_key(|m| {
                if m.contains("opus") { 0 }
                else if m.contains("sonnet") { 1 }
                else if m.contains("haiku") { 2 }
                else { 3 }
            });
        }
        Provider::ChatGpt => {
            sorted.sort_by_key(|m| {
                if m.contains("gpt-4o") { 0 }
                else if m.contains("gpt-4") { 1 }
                else if m.contains("gpt-3.5") { 2 }
                else { 3 }
            });
        }
    }

    Ok(sorted)
}
