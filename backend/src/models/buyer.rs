use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddFundRequest {
    /// Amount of funds to add.
    pub amount: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddFundResponse {
    /// Buyer's ID
    pub id: String,
    /// Current fund
    pub fund: u64,
}
