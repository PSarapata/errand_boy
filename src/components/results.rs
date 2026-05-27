use crate::model::{Listing, ProductGroup};
use crate::LOGO1;
use dioxus::prelude::*;

const MIN_SCORE: u8 = 40;

#[derive(Clone, PartialEq)]
enum SortOrder { Score, PriceAsc, PriceDesc, Alpha }

#[component]
pub fn Results(groups: Vec<ProductGroup>, on_open_url: EventHandler<String>) -> Element {
    let mut hide_out_of_stock = use_signal(|| false);
    let mut min_score = use_signal(|| MIN_SCORE);
    let mut sort = use_signal(|| SortOrder::Score);
    let mut dismissed: Signal<Vec<String>> = use_signal(Vec::new);
    let mut tray_open = use_signal(|| false);

    // Stable group key: brand + model
    let key = |g: &ProductGroup| format!("{} {}", g.brand, g.model);

    let dismissed_keys = dismissed();

    // Split into main (passes score + not dismissed) vs auto-filtered (fails score)
    let mut main: Vec<ProductGroup> = Vec::new();
    let mut auto_filtered: Vec<ProductGroup> = Vec::new();
    let mut manually_dismissed: Vec<ProductGroup> = Vec::new();

    for g in groups.into_iter() {
        let k = key(&g);
        if dismissed_keys.contains(&k) {
            manually_dismissed.push(g);
        } else if g.match_score < min_score() {
            auto_filtered.push(g);
        } else if hide_out_of_stock() && !g.listings.iter().any(|l| l.in_stock.unwrap_or(true)) {
            auto_filtered.push(g);
        } else {
            main.push(g);
        }
    }

    match sort() {
        SortOrder::Score => main.sort_by(|a, b| b.match_score.cmp(&a.match_score)),
        SortOrder::PriceAsc => main.sort_by(|a, b| {
            let pa = a.listings.first().and_then(|l| parse_price(&l.price)).unwrap_or(f64::MAX);
            let pb = b.listings.first().and_then(|l| parse_price(&l.price)).unwrap_or(f64::MAX);
            pa.partial_cmp(&pb).unwrap_or(std::cmp::Ordering::Equal)
        }),
        SortOrder::PriceDesc => main.sort_by(|a, b| {
            let pa = a.listings.first().and_then(|l| parse_price(&l.price)).unwrap_or(0.0);
            let pb = b.listings.first().and_then(|l| parse_price(&l.price)).unwrap_or(0.0);
            pb.partial_cmp(&pa).unwrap_or(std::cmp::Ordering::Equal)
        }),
        SortOrder::Alpha => main.sort_by(|a, b| {
            format!("{} {}", a.brand, a.model).cmp(&format!("{} {}", b.brand, b.model))
        }),
    }

    let count = main.len();
    let count_label = if count == 1 { "1 result".to_string() } else { format!("{} results", count) };
    let tray_count = auto_filtered.len() + manually_dismissed.len();

    rsx! {
        div { class: "results",
            div { class: "results-toolbar",
                span { "{count_label}" }
                div { class: "toolbar-filters",
                    label { class: "filter-toggle",
                        input {
                            r#type: "checkbox",
                            checked: hide_out_of_stock(),
                            onchange: move |e| hide_out_of_stock.set(e.checked()),
                        }
                        " Hide OOS"
                    }
                    label { class: "filter-toggle",
                        "Min "
                        input {
                            r#type: "range",
                            min: "0",
                            max: "100",
                            step: "5",
                            value: "{min_score}",
                            oninput: move |e| {
                                if let Ok(v) = e.value().parse::<u8>() {
                                    min_score.set(v);
                                }
                            }
                        }
                        span { class: "score-value", "{min_score}%" }
                    }
                    div { class: "sort-btns",
                        button {
                            class: if sort() == SortOrder::Score { "sort-btn active" } else { "sort-btn" },
                            onclick: move |_| sort.set(SortOrder::Score),
                            "Match"
                        }
                        button {
                            class: if sort() == SortOrder::PriceAsc { "sort-btn active" } else { "sort-btn" },
                            onclick: move |_| sort.set(SortOrder::PriceAsc),
                            "Price ↑"
                        }
                        button {
                            class: if sort() == SortOrder::PriceDesc { "sort-btn active" } else { "sort-btn" },
                            onclick: move |_| sort.set(SortOrder::PriceDesc),
                            "Price ↓"
                        }
                        button {
                            class: if sort() == SortOrder::Alpha { "sort-btn active" } else { "sort-btn" },
                            onclick: move |_| sort.set(SortOrder::Alpha),
                            "A–Z"
                        }
                    }
                }
            }

            if main.is_empty() {
                div { class: "results-empty",
                    p { "No results to show." }
                }
            }
            div { class: "results-grid",
                for group in main {
                    ProductCard {
                        group: group.clone(),
                        on_open_url: on_open_url.clone(),
                        on_dismiss: {
                            let k = key(&group);
                            move |_| dismissed.write().push(k.clone())
                        },
                    }
                }
            }

            // Filtered tray
            if tray_count > 0 {
                div { class: "filtered-tray",
                    div {
                        class: "tray-header",
                        onclick: move |_| tray_open.toggle(),
                        span { class: "tray-title",
                            if tray_open() { "▾" } else { "▸" }
                            " Filtered ({tray_count})"
                        }
                        span { class: "tray-hint", "items removed by score filter or you" }
                    }
                    if tray_open() {
                        div { class: "tray-body",
                            if !auto_filtered.is_empty() {
                                p { class: "tray-section-label", "Auto-filtered ({auto_filtered.len()})" }
                                div { class: "results-grid",
                                    for group in auto_filtered {
                                        TrayCard {
                                            group: group,
                                            on_open_url: on_open_url.clone(),
                                            on_restore: EventHandler::new(|_| {}),
                                        }
                                    }
                                }
                            }
                            if !manually_dismissed.is_empty() {
                                p { class: "tray-section-label", "Dismissed ({manually_dismissed.len()})" }
                                div { class: "results-grid",
                                    for group in manually_dismissed {
                                        TrayCard {
                                            group: group.clone(),
                                            on_open_url: on_open_url.clone(),
                                            on_restore: {
                                                let k = key(&group);
                                                EventHandler::new(move |_| {
                                                    dismissed.write().retain(|d| d != &k);
                                                })
                                            },
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ProductCard(
    group: ProductGroup,
    on_open_url: EventHandler<String>,
    on_dismiss: EventHandler<()>,
) -> Element {
    let mut expanded = use_signal(|| false);

    let cheapest_url = group.listings.first().map(|l| l.url.clone()).unwrap_or_default();
    let cheapest_price = group.listings.first().map(|l| l.price.clone()).unwrap_or_else(|| "—".to_string());
    let seller_count = group.listings.len();
    let seller_label = if seller_count == 1 { "1 seller".to_string() } else { format!("{} sellers", seller_count) };
    let has_image = group.image.as_deref().map(|s| !s.is_empty()).unwrap_or(false);
    let image_url = group.image.clone().unwrap_or_default();
    let title = format!("{} {}", group.brand, group.model);
    let score_label = format!("{}%", group.match_score);

    rsx! {
        div { class: "product-card",
            div {
                class: "card-thumbnail",
                onclick: {
                    let url = cheapest_url.clone();
                    move |_| on_open_url.call(url.clone())
                },
                if has_image {
                    img { src: "{image_url}", alt: "{title}" }
                } else {
                    img { src: LOGO1, alt: "{title}" }
                }
                button {
                    class: "card-dismiss-btn",
                    title: "Hide this item",
                    onclick: move |e| {
                        e.stop_propagation();
                        on_dismiss.call(());
                    },
                    "✕"
                }
            }
            div { class: "card-body",
                div {
                    class: "card-title",
                    onclick: {
                        let url = cheapest_url.clone();
                        move |_| on_open_url.call(url.clone())
                    },
                    span { class: "card-brand", "{group.brand} " }
                    span { class: "card-model", "{group.model}" }
                }
                div { class: "card-meta",
                    span { class: "card-price", "{cheapest_price}" }
                    span { class: "card-score", "{score_label}" }
                }
                button {
                    class: "card-expand-btn",
                    onclick: move |_| expanded.toggle(),
                    "{seller_label}"
                }
                if expanded() {
                    div { class: "card-listings",
                        for listing in group.listings {
                            ListingRow {
                                listing: listing,
                                on_open_url: on_open_url.clone(),
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn TrayCard(
    group: ProductGroup,
    on_open_url: EventHandler<String>,
    on_restore: EventHandler<()>,
) -> Element {
    let cheapest_url = group.listings.first().map(|l| l.url.clone()).unwrap_or_default();
    let cheapest_price = group.listings.first().map(|l| l.price.clone()).unwrap_or_else(|| "—".to_string());
    let has_image = group.image.as_deref().map(|s| !s.is_empty()).unwrap_or(false);
    let image_url = group.image.clone().unwrap_or_default();
    let title = format!("{} {}", group.brand, group.model);
    let score_label = format!("{}%", group.match_score);

    rsx! {
        div { class: "product-card tray-card",
            div {
                class: "card-thumbnail",
                onclick: {
                    let url = cheapest_url.clone();
                    move |_| on_open_url.call(url.clone())
                },
                if has_image {
                    img { src: "{image_url}", alt: "{title}" }
                } else {
                    img { src: LOGO1, alt: "{title}" }
                }
                button {
                    class: "card-restore-btn",
                    title: "Restore to main list",
                    onclick: move |e| {
                        e.stop_propagation();
                        on_restore.call(());
                    },
                    "↩"
                }
            }
            div { class: "card-body",
                div {
                    class: "card-title",
                    onclick: {
                        let url = cheapest_url.clone();
                        move |_| on_open_url.call(url.clone())
                    },
                    span { class: "card-brand", "{group.brand} " }
                    span { class: "card-model", "{group.model}" }
                }
                div { class: "card-meta",
                    span { class: "card-price", "{cheapest_price}" }
                    span { class: "card-score tray-score", "{score_label}" }
                }
            }
        }
    }
}

#[component]
fn ListingRow(listing: Listing, on_open_url: EventHandler<String>) -> Element {
    let stock_class = match listing.in_stock {
        Some(true) => "stock-yes",
        Some(false) => "stock-no",
        None => "stock-unknown",
    };
    let stock_text = match listing.in_stock {
        Some(true) => "In stock",
        Some(false) => "Out of stock",
        None => "",
    };
    let delivery = listing.delivery.clone().unwrap_or_default();

    rsx! {
        div {
            class: "listing-row",
            onclick: {
                let url = listing.url.clone();
                move |_| on_open_url.call(url.clone())
            },
            span { class: "listing-source", "{listing.source}" }
            span { class: "listing-price", "{listing.price}" }
            if !delivery.is_empty() {
                span { class: "listing-delivery", "{delivery}" }
            }
            if !stock_text.is_empty() {
                span { class: stock_class, "{stock_text}" }
            }
        }
    }
}

fn parse_price(price: &str) -> Option<f64> {
    let digits: String = price.chars()
        .filter(|c| c.is_ascii_digit() || *c == '.' || *c == ',')
        .collect();
    let normalised = digits.replace(',', ".");
    normalised.parse::<f64>().ok()
}
