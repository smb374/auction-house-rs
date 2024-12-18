use serde::{Deserialize, Serialize};
use ulid::Ulid;
use utoipa::ToSchema;

use super::item::ItemRef;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Bid {
    /// User id, hash key
    pub buyer_id: String,
    /// Ulid, range key
    pub id: Ulid,
    /// Create time, in unix timestamp
    pub create_at: u64,
    /// Target item's hash & range key.
    pub item: ItemRef,
    /// Bid amount.
    pub amount: u64,
    /// Is active bid.
    pub is_active: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BidRef {
    /// User id, hash key
    pub buyer_id: String,
    /// Ulid, range key
    pub id: Ulid,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Purchase {
    /// User id, hash key
    pub buyer_id: String,
    /// Ulid, range key
    pub id: Ulid,
    /// Create time, in unix timestamp
    pub create_at: u64,
    /// Purchased Item Referenece
    pub item: ItemRef,
    /// Purchase price
    pub price: u64,
    /// Item sold time
    pub sold_time: u64,
}
