mod agent;
mod components;
mod config;
mod model;
mod search;

use components::{
    debug_panel::{DebugEntry, DebugPanel},
    loading::Loading,
    results::Results,
    search_form::{SearchForm, SearchQuery},
    setup::Setup,
    splash::Splash,
};
use dioxus::prelude::*;
use model::{AppConfig, ProductGroup};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const SEARCH_COOLDOWN: Duration = Duration::from_secs(60);

fn now_ts() -> String {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let s = (ms / 1000) % 86400;
    let h = s / 3600;
    let m = (s % 3600) / 60;
    let sec = s % 60;
    let millis = ms % 1000;
    format!("{:02}:{:02}:{:02}.{:03}", h, m, sec, millis)
}

fn main() {
    // No native menu bar — the default Window/Edit menus offer nothing this app needs.
    dioxus::LaunchBuilder::desktop()
        .with_cfg(dioxus::desktop::Config::new().with_menu(None))
        .launch(App);
}

#[derive(Clone, PartialEq)]
enum AppState {
    Splash,
    /// `Some` when reached from the settings cog — the saved config to prefill and return to.
    Setup(Option<AppConfig>),
    Main(AppConfig),
}

const STYLE: Asset = asset!("assets/style.css");
pub const LOGO1: Asset = asset!("assets/logo1.png");
pub const LOGO2: Asset = asset!("assets/logo2.png");
pub const LOGO_GEMINI: Asset = asset!("assets/gemini.png");
pub const LOGO_CLAUDE: Asset = asset!("assets/claude.png");
pub const LOGO_CHATGPT: Asset = asset!("assets/chatgpt.png");
pub const COIN: Asset = asset!("assets/coin.png");

#[component]
fn App() -> Element {
    let mut state = use_signal(|| AppState::Splash);
    let mut searching = use_signal(|| false);
    let mut results: Signal<Vec<ProductGroup>> = use_signal(Vec::new);
    let mut search_error = use_signal(String::new);
    let mut last_search: Signal<Option<Instant>> = use_signal(|| None);
    let mut debug_entries: Signal<Vec<DebugEntry>> = use_signal(Vec::new);
    let mut debug_open = use_signal(|| false);
    let mut form_collapsed = use_signal(|| false);

    rsx! {
        document::Stylesheet { href: STYLE }

        match state() {
            AppState::Splash => rsx! {
                Splash {
                    on_done: move |_| {
                        let next = match config::load() {
                            Some(cfg) => AppState::Main(cfg),
                            None => AppState::Setup(None),
                        };
                        state.set(next);
                    }
                }
            },
            AppState::Setup(existing) => rsx! {
                Setup {
                    existing: existing.clone(),
                    on_done: move |cfg| state.set(AppState::Main(cfg)),
                    on_cancel: move |_| {
                        if let Some(cfg) = existing.clone() {
                            state.set(AppState::Main(cfg));
                        }
                    },
                }
            },
            AppState::Main(cfg) => rsx! {
                div { class: "app-layout",
                    div { class: "main-content",
                        SearchForm {
                            loading: searching(),
                            collapsed: form_collapsed() && !searching(),
                            on_expand: move |_| form_collapsed.set(false),
                            cooldown_secs: last_search().and_then(|t| {
                                let elapsed = t.elapsed();
                                if elapsed < SEARCH_COOLDOWN {
                                    Some((SEARCH_COOLDOWN - elapsed).as_secs() as u32)
                                } else {
                                    None
                                }
                            }),
                            on_settings: {
                                let cfg = cfg.clone();
                                move |_| state.set(AppState::Setup(Some(cfg.clone())))
                            },
                            on_search: move |q: SearchQuery| {
                                let cfg = cfg.clone();
                                async move {
                                    let ts = now_ts();
                                    let query_label = q.query[..q.query.len().min(40)].to_string();

                                    // Log the button click itself
                                    debug_entries.write().push(DebugEntry {
                                        timestamp: ts.clone(),
                                        label: format!("CLICK: {}", query_label),
                                        request: format!(
                                            "category: {}\nquery: {}\nrefinement: {}",
                                            q.category,
                                            q.query,
                                            q.refinement.as_deref().unwrap_or("—")
                                        ),
                                        response: String::new(),
                                        ok: true,
                                    });

                                    searching.set(true);
                                    search_error.set(String::new());
                                    results.set(Vec::new());
                                    last_search.set(Some(Instant::now()));

                                    let outcome = run_search_debug(&cfg, &q).await;

                                    let serp_ok = outcome.serp_ok;
                                    let agent_ok = outcome.agent_ok;
                                    debug_entries.write().push(DebugEntry {
                                        timestamp: now_ts(),
                                        label: format!("SERP: {}", query_label),
                                        request: outcome.serp_req,
                                        response: outcome.serp_resp,
                                        ok: serp_ok,
                                    });
                                    debug_entries.write().push(DebugEntry {
                                        timestamp: now_ts(),
                                        label: format!("LLM: {}", query_label),
                                        request: outcome.agent_req,
                                        response: outcome.agent_resp,
                                        ok: agent_ok,
                                    });

                                    if !serp_ok || !agent_ok {
                                        debug_open.set(true);
                                    }

                                    match outcome.groups {
                                        Ok(groups) => {
                                            results.set(groups);
                                            form_collapsed.set(true);
                                        }
                                        Err(e) => {
                                            search_error.set(e);
                                            debug_open.set(true);
                                        }
                                    }
                                    searching.set(false);
                                }
                            }
                        }

                        if !search_error().is_empty() {
                            div { class: "search-error", "{search_error}" }
                        }

                        if searching() {
                            Loading {}
                        } else if !results().is_empty() {
                            Results {
                                groups: results(),
                                on_open_url: move |url: String| {
                                    let _ = open::that(&url);
                                }
                            }
                        }
                    }

                    DebugPanel {
                        entries: debug_entries(),
                        open: debug_open(),
                        on_toggle: move |_| debug_open.set(!debug_open()),
                        on_clear: move |_| debug_entries.set(Vec::new()),
                    }
                }
            },
        }
    }
}

fn inject_images(
    mut groups: Vec<ProductGroup>,
    serp: &[search::SearchResult],
) -> Vec<ProductGroup> {
    // Build URL → image index from SERP
    let url_image: std::collections::HashMap<&str, &str> = serp
        .iter()
        .filter_map(|r| r.image.as_deref().map(|img| (r.url.as_str(), img)))
        .collect();

    for group in &mut groups {
        // Step 1: URL match — any listing URL present in SERP map
        let image = group.listings.iter().find_map(|l| {
            url_image.get(l.url.as_str()).copied()
        });

        // Step 2: title match — SERP title must contain BOTH a brand word AND a model word
        // Use only words longer than 3 chars to avoid matching "HPI", "RC", etc.
        let image = image.or_else(|| {
            let brand_words: Vec<&str> = group.brand.split_whitespace()
                .filter(|w| w.len() > 3).collect();
            let model_words: Vec<&str> = group.model.split_whitespace()
                .filter(|w| w.len() > 3).collect();

            // Need at least one meaningful word from each side
            if brand_words.is_empty() || model_words.is_empty() {
                return None;
            }

            serp.iter().find_map(|r| {
                r.image.as_ref()?;
                let t = r.title.to_lowercase();
                let brand_hit = brand_words.iter().any(|w| t.contains(&w.to_lowercase()));
                let model_hit = model_words.iter().any(|w| t.contains(&w.to_lowercase()));
                if brand_hit && model_hit { r.image.as_deref() } else { None }
            })
        });

        if let Some(img) = image {
            group.image = Some(img.to_string());
        }
    }
    groups
}

struct SearchOutcome {
    serp_req: String, serp_resp: String, serp_ok: bool,
    agent_req: String, agent_resp: String, agent_ok: bool,
    groups: Result<Vec<ProductGroup>, String>,
}

async fn run_search_debug(cfg: &AppConfig, q: &SearchQuery) -> SearchOutcome {
    // Phase 1: SERP call
    let (search_result, serp_req, serp_resp) = search::search(
        &cfg.search_api_key,
        &q.query,
        q.exclude.as_deref(),
        q.location.as_deref(),
        q.pages,
    )
    .await;

    let serp_ok = search_result.is_ok();

    // Phase 2: LLM analyzes and groups results
    let (agent_req, agent_resp, agent_ok, groups) = match &search_result {
        Err(e) => (String::new(), String::new(), false, Err(e.clone())),
        Ok(serp_results) => {
            let formatted = search::format_for_prompt(serp_results);
            let (res, req, resp) = agent::run(
                &cfg.llm_provider,
                &cfg.llm_api_key,
                &cfg.llm_model,
                Some(&q.category),
                &q.query,
                q.refinement.as_deref(),
                &formatted,
            )
            .await;
            let ok = res.is_ok();
            let mapped = res.map(|r| inject_images(r.groups, serp_results));
            (req, resp, ok, mapped)
        }
    };

    SearchOutcome {
        serp_req, serp_resp, serp_ok,
        agent_req, agent_resp, agent_ok,
        groups,
    }
}
