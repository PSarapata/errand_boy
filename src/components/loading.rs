use crate::COIN;
use dioxus::prelude::*;

#[component]
pub fn Loading() -> Element {
    rsx! {
        div { class: "loading-overlay",
            div { class: "coin-container",
                img {
                    src: COIN,
                    class: "coin-image",
                    alt: "Searching…"
                }
            }
            p { class: "loading-label", "Searching…" }
        }
    }
}
