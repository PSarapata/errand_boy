# Errand Boy — Agent Instructions

## Project layout

| Path | Responsibility |
|------|---------------|
| `src/main.rs` | App root, state machine (`AppState`), search orchestration (`run_search_debug`) |
| `src/components/` | All Dioxus UI components. One file per component or tightly-related group. |
| `src/agent/mod.rs` | LLM API calls. `run()` analyses SERP results. Supports Gemini, Claude, ChatGPT. |
| `src/search/mod.rs` | Value SERP API client. `search()` fetches results; `format_for_prompt()` formats for LLM. |
| `src/model.rs` | Shared data types (`ProductGroup`, `Listing`, `AppConfig`, `SearchResponse`). |
| `src/config.rs` | Config persistence (`~/.config/errand_boy/config.toml`). |
| `assets/` | Static assets (CSS, images). CSS is the single stylesheet — no CSS modules. |

## Search pipeline (three stages)

1. **SERP** (`src/search/mod.rs`) — parallel page fetches from Value SERP shopping API. Negative keywords appended to query as `-term`. Returns `Vec<SearchResult>`.
2. **LLM** (`src/agent/mod.rs`) — formats SERP results and sends to configured LLM provider. Returns `Vec<ProductGroup>` (grouped by brand+model, scored 0–100).
3. **UI** (`src/components/results.rs`) — renders groups as cards. User can dismiss cards to the filtered tray; auto-filtered groups (below min-score) also land in the tray.

## Component conventions

- Components are `#[component]` functions returning `Element`.
- State lives in `use_signal`. Signals are passed down as props — no global state store.
- Event callbacks are `EventHandler<T>` props, named `on_*`.
- UI-only logic stays in the component file. Network/API logic belongs in `src/agent/` or `src/search/`.

## LLM provider pattern

`agent::run()` dispatches to `call_claude / call_gemini / call_chatgpt` based on a `provider: &str` string (`"gemini"`, `"chatgpt"`, anything else → claude). Adding a new provider means adding a new `call_*` function and a new arm in the `match provider` block.

## Naming conventions

- Files: `snake_case.rs`
- Components and structs: `PascalCase`
- Functions and variables: `snake_case`
- CSS classes: `kebab-case`
- Signals: named for what they hold, not `*_signal` suffix (e.g., `results`, not `results_signal`)

## CSS

Single stylesheet at `assets/style.css`. Dark theme (`#1a1a2e` background family). No framework — vanilla CSS with BEM-like class naming. Add new styles at the bottom of the relevant section, or create a new section with a `/* ── Section Name ── */` comment.

## What NOT to do

- Do not add runtime reflection or dynamic dispatch where Rust's type system can express the contract statically.
- Do not add `unwrap()` on `Result` or `Option` in production paths — use `?`, `map_err`, or explicit fallback.
- Do not create new files for single-use helpers — colocate with the calling module unless reused across two or more modules.
