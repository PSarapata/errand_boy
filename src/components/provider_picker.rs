use crate::{LOGO_CHATGPT, LOGO_CLAUDE, LOGO_GEMINI};
use dioxus::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub enum Provider {
    Gemini,
    Claude,
    ChatGpt,
}

impl Provider {
    pub fn id(&self) -> &'static str {
        match self {
            Provider::Gemini => "gemini",
            Provider::Claude => "claude",
            Provider::ChatGpt => "chatgpt",
        }
    }

    pub fn from_id(id: &str) -> Option<Provider> {
        match id {
            "gemini" => Some(Provider::Gemini),
            "claude" => Some(Provider::Claude),
            "chatgpt" => Some(Provider::ChatGpt),
            _ => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Provider::Gemini => "Gemini",
            Provider::Claude => "Claude",
            Provider::ChatGpt => "ChatGPT",
        }
    }

    pub fn key_placeholder(&self) -> &'static str {
        match self {
            Provider::Gemini => "AIza...",
            Provider::Claude => "sk-ant-...",
            Provider::ChatGpt => "sk-...",
        }
    }

    pub fn api_key_url(&self) -> &'static str {
        match self {
            Provider::Gemini => "https://aistudio.google.com/",
            Provider::Claude => "https://platform.claude.com/settings/keys",
            Provider::ChatGpt => "https://platform.openai.com/api-keys",
        }
    }

    pub fn models_url(&self) -> &'static str {
        match self {
            Provider::Gemini => "https://generativelanguage.googleapis.com/v1beta/models",
            Provider::Claude => "https://api.anthropic.com/v1/models",
            Provider::ChatGpt => "https://api.openai.com/v1/models",
        }
    }
}

#[component]
pub fn ProviderPicker(on_pick: EventHandler<Provider>) -> Element {
    let mut hovered = use_signal(|| Option::<Provider>::None);

    rsx! {
        div { class: "provider-screen",
            h2 { class: "provider-title", "Choose your AI provider" }
            p { class: "provider-subtitle", "Gemini is recommended — it has a free API tier." }

            div { class: "trefoil",
                // Top — Gemini
                div {
                    class: if hovered() == Some(Provider::Gemini) { "provider-card hovered gemini" } else { "provider-card gemini" },
                    style: "top: 0; left: 50%; transform: translateX(-50%);",
                    onmouseenter: move |_| hovered.set(Some(Provider::Gemini)),
                    onmouseleave: move |_| hovered.set(None),
                    onclick: move |_| on_pick.call(Provider::Gemini),
                    img { src: LOGO_GEMINI, alt: "Gemini" }
                    span { class: "provider-badge", "Free tier" }
                }

                // Bottom-left — Claude
                div {
                    class: if hovered() == Some(Provider::Claude) { "provider-card hovered claude" } else { "provider-card claude" },
                    style: "bottom: 0; left: 0;",
                    onmouseenter: move |_| hovered.set(Some(Provider::Claude)),
                    onmouseleave: move |_| hovered.set(None),
                    onclick: move |_| on_pick.call(Provider::Claude),
                    img { src: LOGO_CLAUDE, alt: "Claude" }
                }

                // Bottom-right — ChatGPT
                div {
                    class: if hovered() == Some(Provider::ChatGpt) { "provider-card hovered chatgpt" } else { "provider-card chatgpt" },
                    style: "bottom: 0; right: 0;",
                    onmouseenter: move |_| hovered.set(Some(Provider::ChatGpt)),
                    onmouseleave: move |_| hovered.set(None),
                    onclick: move |_| on_pick.call(Provider::ChatGpt),
                    img { src: LOGO_CHATGPT, alt: "ChatGPT" }
                }
            }
        }
    }
}
