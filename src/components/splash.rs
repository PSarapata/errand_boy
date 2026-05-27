use crate::LOGO2;
use dioxus::prelude::*;

const SPLASH_DURATION_MS: u64 = 2000;

#[component]
pub fn Splash(on_done: EventHandler<()>) -> Element {
    use_future(move || async move {
        tokio::time::sleep(std::time::Duration::from_millis(SPLASH_DURATION_MS)).await;
        on_done.call(());
    });

    rsx! {
        div {
            style: "display:flex; justify-content:center; align-items:center; height:100vh; background:#1a1a1a;",
            img {
                src: LOGO2,
                style: "max-width:480px; width:80%;",
                alt: "Errand Boy"
            }
        }
    }
}
