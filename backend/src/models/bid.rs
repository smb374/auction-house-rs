use serde::{Deserialize, Serialize};
use ulid::Ulid;
use utoipa::ToSchema;

use super::item::ItemRef;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Bid {
    /// User id, hash key
    buyer_id: String,
    /// Ulid, range key
    id: Ulid,
    /// Create time, in unix timestamp
    create_at: u64,
    /// Target item's hash & range key.
    item: ItemRef,
    /// Bid amount.
    amount: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BidRef {
    /// User id, hash key
    buyer_id: String,
    /// Ulid, range key
    id: Ulid,
}
