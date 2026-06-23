//! Market events - the unit of data produced by exchange connectors.
//!
//! Every consumer (strategy, book builder, recorder) receives `MarketEvent`.
//! Adding a new data type = adding a variant here. The compiler's exhaustive
//! match ensures you handle it everywhere.
//!
//! Two timestamps on every event:
//! - `exchange_ts` — when it happened at the exchange
//! - `local_ts`    — when our system received it
//!
//! Their difference = your network latency. Track it from day one.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::instrument::Instrument;
use crate::primitives::{Price, Qty};

/// Buy or sell side of a trade or order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Buy,
    Sell,
}

impl Side {
    pub fn opposite(&self) -> Side {
        match self {
            Side::Buy => Side::Sell,
            Side::Sell => Side::Buy,
        }
    }
}

impl std::fmt::Display for Side {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Side::Buy => write!(f, "buy"),
            Side::Sell => write!(f, "sell"),
        }
    }
}

/// A single public market trade (a "tick").
/// This is NOT our fill - it's a trade visible to everyone.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub trade_id: Option<String>,
    pub price: Price,
    pub qty: Qty,
    /// Who crossed the spread - the taker.
    pub aggressor_side: Side,
    pub exchange_ts: OffsetDateTime,
    pub local_ts: OffsetDateTime,
}

/// One price level in the order book: price + total resting qty.
/// qty == 0 means this level was deleted - remove it from your local book.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookLevel {
    pub price: Price,
    pub qty: Qty,
}

/// An incremental or snapshot update to the order book.
///
/// `is_snapshot = true`  → replace your entire local book with this data.
/// `is_snapshot = false` → apply these deltas; levels with qty=0 are deletions.
///
/// `sequence` must be monotonically increasing. A gap means you missed
/// messages and must re-sync by requesting a fresh snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookUpdate {
    pub bids: Vec<BookLevel>,
    pub asks: Vec<BookLevel>,
    pub is_snapshot: bool,
    pub sequence: u64,
    pub exchange_ts: OffsetDateTime,
    pub local_ts: OffsetDateTime,
}

/// The central event type. Every component in the system speaks this.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketEvent {
    pub instrument: Instrument,
    pub kind: MarketEventKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MarketEventKind {
    Trade(Trade),
    OrderBookUpdate(OrderBookUpdate),
}

impl MarketEvent {
    pub fn new(instrument: Instrument, kind: MarketEventKind) -> Self {
        Self { instrument, kind }
    }
}
