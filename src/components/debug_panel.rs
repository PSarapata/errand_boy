use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
pub struct DebugEntry {
    pub timestamp: String,
    pub label: String,
    pub request: String,
    pub response: String,
    pub ok: bool,
}

#[component]
pub fn DebugPanel(entries: Vec<DebugEntry>, open: bool, on_clear: EventHandler<()>, on_toggle: EventHandler<()>) -> Element {
    let mut open_idx: Signal<Option<usize>> = use_signal(|| None);

    rsx! {
        div { class: if open { "debug-panel open" } else { "debug-panel" },
            div {
                class: "debug-tab",
                onclick: move |_| on_toggle.call(()),
                if open { "▶" } else { "◀" }
            }

            if open {
                div { class: "debug-header",
                    span { "Debug" }
                    button {
                        class: "debug-btn",
                        onclick: move |_| on_clear.call(()),
                        "Clear"
                    }
                }

                div { class: "debug-entries",
                    if entries.is_empty() {
                        p { class: "debug-empty", "No requests yet." }
                    }
                    for (i, entry) in entries.iter().enumerate() {
                        div { class: "debug-entry",
                            div {
                                class: if entry.ok { "debug-entry-header ok" } else { "debug-entry-header err" },
                                onclick: move |_| {
                                    let next = if open_idx() == Some(i) { None } else { Some(i) };
                                    open_idx.set(next);
                                },
                                span { class: "debug-status", if entry.ok { "✓" } else { "✗" } }
                                span { class: "debug-ts", "{entry.timestamp}" }
                                span { class: "debug-label", "{entry.label}" }
                                span { class: "debug-chevron", if open_idx() == Some(i) { "▾" } else { "▸" } }
                            }
                            if open_idx() == Some(i) {
                                div { class: "debug-body",
                                    div { class: "debug-section-header",
                                        p { class: "debug-section-title", "REQUEST" }
                                        {copy_btn(entry.request.clone())}
                                    }
                                    pre { class: "debug-pre", "{entry.request}" }
                                    div { class: "debug-section-header",
                                        p { class: "debug-section-title", "RESPONSE" }
                                        {copy_btn(entry.response.clone())}
                                    }
                                    pre { class: "debug-pre", "{entry.response}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn copy_btn(text: String) -> Element {
    let mut copied = use_signal(|| false);

    rsx! {
        button {
            class: if copied() { "debug-copy-btn copied" } else { "debug-copy-btn" },
            title: "Copy to clipboard",
            onclick: move |_| {
                let script = format!(
                    "navigator.clipboard.writeText({})",
                    serde_json::to_string(&text).unwrap_or_default()
                );
                document::eval(&script);
                copied.set(true);
            },
            if copied() { "✓" } else { "⎘" }
        }
    }
}
