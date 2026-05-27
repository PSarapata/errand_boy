use dioxus::prelude::*;

const CATEGORIES: &[&str] = &[
    "General",
    "Hobby & RC",
    "Electronics",
    "Tools & Hardware",
    "Sports & Outdoors",
    "Automotive",
    "Home & Garden",
    "Collectibles",
];

const PAGES: &[u32] = &[1, 2, 3, 4, 5];

// (display label, Value SERP location string)
const LOCATIONS: &[(&str, &str)] = &[
    ("Worldwide", ""),
    ("United States", "United States"),
    ("European Union", "European Union"),
    ("United Kingdom", "United Kingdom"),
    ("Germany", "Germany"),
    ("France", "France"),
    ("Poland", "Poland"),
    ("Netherlands", "Netherlands"),
    ("Spain", "Spain"),
    ("Italy", "Italy"),
    ("Canada", "Canada"),
    ("Australia", "Australia"),
    ("Japan", "Japan"),
];

#[derive(Debug, Clone, PartialEq)]
pub struct SearchQuery {
    pub category: String,
    pub query: String,
    pub exclude: Option<String>,
    pub refinement: Option<String>,
    pub pages: u32,
    pub location: Option<String>,
}

#[component]
pub fn SearchForm(
    on_search: EventHandler<SearchQuery>,
    on_settings: EventHandler<()>,
    loading: bool,
    cooldown_secs: Option<u32>,
    collapsed: bool,
    on_expand: EventHandler<()>,
) -> Element {
    let mut category = use_signal(|| CATEGORIES[0].to_string());
    let mut query = use_signal(String::new);
    let mut exclude_chips: Signal<Vec<String>> = use_signal(Vec::new);
    let mut exclude_input = use_signal(String::new);
    let mut refinement = use_signal(String::new);
    let mut pages = use_signal(|| PAGES[0]);
    let mut location = use_signal(|| LOCATIONS[0].1.to_string());

    let mut commit_chip = move || {
        let val = exclude_input().trim().trim_start_matches('-').to_string();
        if !val.is_empty() && !exclude_chips().contains(&val) {
            exclude_chips.write().push(val);
        }
        exclude_input.set(String::new());
    };

    let on_cooldown = cooldown_secs.is_some();
    let can_submit = !loading && !on_cooldown && !query().trim().is_empty();
    let btn_label = if loading {
        "Searching…".to_string()
    } else if let Some(secs) = cooldown_secs {
        format!("Wait {secs}s…")
    } else {
        "Search".to_string()
    };

    if collapsed {
        let q = query();
        let summary = if q.trim().is_empty() { "New search".to_string() } else { q };
        return rsx! {
            div {
                class: "search-form search-form-collapsed",
                onclick: move |_| on_expand.call(()),
                span { class: "search-summary", "🔍 {summary}" }
                span { class: "search-expand-hint", "click to edit" }
            }
        };
    }

    rsx! {
        div { class: "search-form",
            div { class: "search-header",
                button {
                    class: "settings-btn",
                    title: "Settings",
                    onclick: move |_| on_settings.call(()),
                    "⚙"
                }
            }

            div { class: "form-row",
                label { "Category" }
                select {
                    value: "{category}",
                    onchange: move |e| category.set(e.value()),
                    for cat in CATEGORIES.iter() {
                        option { value: "{cat}", "{cat}" }
                    }
                }
            }

            div { class: "form-row",
                label { "What are you looking for?" }
                input {
                    r#type: "text",
                    placeholder: "e.g. 1/10 200mm touring car nitro bodyshell",
                    value: "{query}",
                    oninput: move |e| query.set(e.value()),
                    onkeydown: move |e| {
                        if e.key() == Key::Enter && can_submit {
                            let q = build_query(&category, &query, &exclude_chips, &refinement, &pages, &location);
                            on_search.call(q);
                        }
                    }
                }
            }

            div { class: "form-row",
                label { "Exclude terms (optional)" }
                div { class: "chip-input-wrapper",
                    for chip in exclude_chips() {
                        span { class: "exclude-chip",
                            "-{chip}"
                            button {
                                class: "chip-remove",
                                onclick: {
                                    let chip = chip.clone();
                                    move |e: Event<MouseData>| {
                                        e.stop_propagation();
                                        exclude_chips.write().retain(|c| c != &chip);
                                    }
                                },
                                "×"
                            }
                        }
                    }
                    input {
                        r#type: "text",
                        class: "chip-input",
                        placeholder: if exclude_chips().is_empty() { "e.g. drift, s13, casual" } else { "" },
                        value: "{exclude_input}",
                        oninput: move |e| exclude_input.set(e.value()),
                        onblur: move |_| commit_chip(),
                        onkeydown: move |e| {
                            match e.key() {
                                Key::Enter => {
                                    commit_chip();
                                }
                                Key::Tab => {
                                    commit_chip();
                                }
                                Key::Backspace => {
                                    if exclude_input().is_empty() {
                                        exclude_chips.write().pop();
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }

            div { class: "form-row",
                label { "Refine results (optional)" }
                input {
                    r#type: "text",
                    placeholder: "e.g. prioritize rarity, drop out of stock items",
                    value: "{refinement}",
                    oninput: move |e| refinement.set(e.value()),
                    onkeydown: move |e| {
                        if e.key() == Key::Enter && can_submit {
                            let q = build_query(&category, &query, &exclude_chips, &refinement, &pages, &location);
                            on_search.call(q);
                        }
                    }
                }
            }

            div { class: "form-row-split",
                div { class: "form-row",
                    label { "Location" }
                    select {
                        value: "{location}",
                        onchange: move |e| location.set(e.value()),
                        for (label, val) in LOCATIONS.iter() {
                            option { value: "{val}", "{label}" }
                        }
                    }
                }
                div { class: "form-row",
                    label { "Pages (~40 results each)" }
                    select {
                        value: "{pages}",
                        onchange: move |e| {
                            if let Ok(n) = e.value().parse::<u32>() {
                                pages.set(n);
                            }
                        },
                        for n in PAGES.iter() {
                            option { value: "{n}", "{n}" }
                        }
                    }
                }
            }

            button {
                class: "search-btn",
                disabled: !can_submit,
                onclick: move |_| {
                    let q = build_query(&category, &query, &exclude_chips, &refinement, &pages, &location);
                    on_search.call(q);
                },
                "{btn_label}"
            }
        }
    }
}

fn build_query(
    category: &Signal<String>,
    query: &Signal<String>,
    exclude_chips: &Signal<Vec<String>>,
    refinement: &Signal<String>,
    pages: &Signal<u32>,
    location: &Signal<String>,
) -> SearchQuery {
    let r = refinement().trim().to_string();
    let loc = location().trim().to_string();
    let chips = exclude_chips();
    SearchQuery {
        category: category(),
        query: query().trim().to_string(),
        exclude: if chips.is_empty() { None } else { Some(chips.join(",")) },
        refinement: if r.is_empty() { None } else { Some(r) },
        pages: pages(),
        location: if loc.is_empty() { None } else { Some(loc) },
    }
}
