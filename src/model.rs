use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Listing {
    pub source: String,
    pub price: String,
    pub url: String,
    pub delivery: Option<String>,
    pub in_stock: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProductGroup {
    pub brand: String,
    pub model: String,
    pub image: Option<String>,
    pub match_score: u8,
    pub listings: Vec<Listing>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub groups: Vec<ProductGroup>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppConfig {
    pub llm_provider: String,
    pub llm_model: String,
    pub llm_api_key: String,
    pub search_api_key: String,
}
