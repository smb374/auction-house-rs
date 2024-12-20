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

impl From<&Bid> for BidRef {
    fn from(value: &Bid) -> Self {
        Self {
            buyer_id: value.buyer_id.clone(),
            id: value.id,
        }
    }
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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BidItemRequest {
    /// Seller ID of the item
    pub seller_id: String,
    /// ID of the item
    pub id: Ulid,
    /// Bid amount
    pub amount: u64,
}
