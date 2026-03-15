use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCatalogEntry {
    pub id: String,
    pub provider: String,
    pub model_id: String,
    pub display_name: String,
    pub cost_per_input_token: Option<f64>,
    pub cost_per_output_token: Option<f64>,
    pub context_window: Option<i64>,
    pub category: String,
    pub strengths: Option<String>,
    pub is_available: bool,
    pub updated_at: i64,
}
