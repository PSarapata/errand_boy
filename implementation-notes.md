> Product requirements: see `/home/psarapata/projects/10xDevs/m1l1/context/foundation/prd.md`

# 1. Claude model
The user selects the model during initial setup (or from settings). On API key entry, fetch the available models from the Anthropic API (`GET /v1/models`) and populate a dropdown. Default selection: the most capable model in the returned list (sort by capability tier — Opus > Sonnet > Haiku).

Store the selected model ID in local config alongside the API key. Pass it as the `model` parameter on every request — never hardcode a model ID in the agent code.

# 2. Project structure conventions

Single Cargo workspace. Suggested crate layout:

```
errand_boy/
  Cargo.toml              # workspace root
  src/
    main.rs               # Dioxus app entry point
    components/           # UI components (search form, result card, setup screen)
    agent/                # prompt building, LLM call, response parsing
    search/               # Google Custom Search API client
    config.rs             # local config load/save (API keys, selected provider)
    model.rs              # shared data types (SearchResult, ProductGroup, etc.)
```

Config/keys storage: use the OS-standard config directory (`dirs` crate — `config_dir()`) so keys are not stored next to the binary. File format: TOML. Never store keys in the repo.


# 3. Prompt design
Structure of the prompt that will be sent to Claude (how search results are passed, what the expected output schema is). Example search agent prompt:

You are a personal shopping assistant,
[Optional] an expert in the {category} branch.
The user is searching for:
{user_query}
[Optional] Limit your search to {region} region.
[Optional] User left you instructions, to further refine query results based on this criteria: {user_query2}

Use the search results provided below, analyze them and group by brand and model (different sellers offering the same product are one group), then return a JSON object matching this structure exactly:

```json
{
  "groups": [
    {
      "brand": "Brand name",
      "model": "Model name",
      "image": "Product image URL from the best-matching listing",
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
```

Rules:
- One group per distinct brand+model combination. Do not create separate groups for the same product sold by different stores.
- match_score (0–100) reflects how closely this product matches the user's query.
- Sort groups by match_score descending.
- Within each group, sort listings by price ascending (lowest price first). The first listing is always the cheapest — the UI uses listings[0].url as the primary click-through link.
- If stock status cannot be determined from the result, set in_stock to null (not false).
- Return only the JSON object. No explanation, no markdown fencing.

Search results:
{search_results}

# 4. Value SERP setup details

API reference: https://www.valueserp.com/docs/search-api/overview

One credential required (stored in local config, never hardcoded):
- **API key** — from https://app.valueserp.com/signup (free tier available)

Endpoint: `https://api.valueserp.com/search?api_key={api_key}&search_type=shopping&q={query}&num=10`

- Use `search_type=shopping` to get structured product results in `shopping_results[]`.
- Optionally pass `location` for region-scoped results (maps from the refinement instruction if the user specifies a region).
- Free tier has a request quota; surface an error if the API returns a 429.

**Fields to extract from each `shopping_results[]` entry:**

| Field | Notes |
|---|---|
| `title` | Product name |
| `merchant` | Store/seller name |
| `link` | Direct URL to listing |
| `price` | Price with currency symbol (e.g. "£18.99") |
| `price_parsed` | Numeric value — use for sorting if needed |
| `image` | Product image URL — pass through to UI directly (no image_hint needed) |
| `delivery` | Shipping info if present (e.g. "Free delivery") |

Pass extracted fields as plain text in the `{search_results}` slot — not raw JSON. Format each result as:

```
[1] title: HPI Racing 1/10 Nitro RS4 3 Evo+ Body Shell
    store: HPI Racing
    url: https://...
    price: £18.99
    delivery: Free delivery

[2] ...
```

Omit `delivery` line if absent — do not write "N/A".

# 5. UI fidelity expectations
Each result card displays:
- Thumbnail (real product image from `image` URL; fall back to `assets/logo1.png` if absent)
- Brand + model name
- Lowest price across listings (always `listings[0].price` — the prompt guarantees ascending price sort)
- Number of sellers found (e.g. "4 sellers")
- match_score as a visual indicator (e.g. badge or progress bar)

**Card click behaviour (two actions):**
- **Primary (single click / main click target):** open `listings[0].url` in the system browser — the cheapest listing, directly.
- **Secondary (expand toggle):** reveal a dropdown list of all listings for the group, each showing source, price, delivery info, and in_stock status, each linking to its own URL.

Out-of-stock filter: global toggle, off by default (all results shown). When enabled, hides groups where all listings have in_stock = false. Groups with in_stock = null are shown regardless (unknown ≠ out of stock).