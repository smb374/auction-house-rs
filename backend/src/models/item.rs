use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::bid::BidRef;

/// Item State Enum
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ItemState {
    Active,
    Archived,
    Completed,
    Failed,
    InActive,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    /// User id, hash key
    seller_id: String,
    /// Ulid inner repr, range key
    id: u128,
    /// Create time, in unix timestamp
    create_at: u64,
    /// Item Name
    name: String,
    /// Item Description
    description: String,
    /// Initial Price, >1,
    init_price: u64,
    /// Item state, see enum def.
    state: ItemState,
    /// Length of Auction, in unix timestamp diff.
    auction_length: u64,
    /// List of S3 keys
    images: Vec<String>,
    /// Unix timestamp, Some when item_state == "active"
    start_date: Option<u64>,
    /// Unix timestamp, Some when item_state == "active"
    end_date: Option<u64>,
    /// Current bid's hash & range key.
    current_bid: BidRef,
    /// List of past bids' hash & range key.
    past_bids: Vec<BidRef>,
    /// Item sold bid
    sold_bid: Option<BidRef>,
    /// Item sold unixtimestamp
    sold_time: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ItemRef {
    // User id, hash key
    seller_id: String,
    // Ulid inner repr, range key
    id: u128,
}
