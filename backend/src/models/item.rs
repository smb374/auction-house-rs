use core::fmt;

use serde::{Deserialize, Serialize};
use ulid::Ulid;
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

impl Default for ItemState {
    fn default() -> Self {
        Self::InActive
    }
}

impl fmt::Display for ItemState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let out = match *self {
            ItemState::Active => "active",
            ItemState::Archived => "archived",
            ItemState::Completed => "completed",
            ItemState::Failed => "failed",
            ItemState::InActive => "inactive",
        };
        write!(f, "{}", out)
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    /// User id, hash key
    seller_id: String,
    /// Ulid, range key
    id: Ulid,
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
    current_bid: Option<BidRef>,
    /// List of past bids' hash & range key.
    past_bids: Vec<BidRef>,
    /// Item sold bid
    sold_bid: Option<BidRef>,
    /// Item sold unixtimestamp
    sold_time: Option<u64>,
}

impl Item {
    pub fn new_from_request(seller_id: String, req: AddItemRequest) -> Self {
        Self {
            seller_id,
            id: Ulid::new(),
            create_at: chrono::Local::now().timestamp_millis() as u64,
            name: req.name,
            description: req.description,
            init_price: req.init_price,
            auction_length: req.auction_length,
            images: req.images,
            ..Default::default()
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ItemRef {
    // User id, hash key
    seller_id: String,
    // Ulid, range key
    id: Ulid,
}

impl From<&Item> for ItemRef {
    fn from(value: &Item) -> Self {
        Self {
            seller_id: value.seller_id.clone(),
            id: value.id,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddItemRequest {
    /// Item Name
    name: String,
    /// Item Description
    description: String,
    /// Initial Price, >1,
    init_price: u64,
    /// Length of Auction, in unix timestamp diff.
    auction_length: u64,
    /// List of S3 keys
    images: Vec<String>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateItemRequest {
    /// Item Name
    pub name: Option<String>,
    /// Item Description
    pub description: Option<String>,
    /// Initial Price, >1,
    pub init_price: Option<u64>,
    /// Length of Auction, in unix timestamp diff.
    pub auction_length: Option<u64>,
    /// List of S3 keys
    pub images: Option<Vec<String>>,
}
