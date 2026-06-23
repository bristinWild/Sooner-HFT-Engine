//! Local order book - maintains the full bid/ask state from snapshots + deltas.
//!
//! ## How it works
//!
//! Bybit sends one snapshot on subscribe, then a stream of deltas.
//! We store levels in two `BTreeMap<Price, Qty>` - one for bids, one for asks.
//!
//! BTreeMap is the right structure here because:
//! - Keys (prices) are always sorted - best bid = last key, best ask = first key
//! - Insert/delete/lookup are all O(log n) - fine for 50 levels
//! - Iteration in price order is free
//!
//! ## Sequence validation
//!
//! Every update carries a `u` (update ID). We check that each delta's ID
//! is exactly `last_seq + 1`. A gap means we missed messages and the book
//! is stale - we must discard it and wait for the next snapshot.

use std::collections::BTreeMap;

use tracing::{debug, warn};

use hft_core::{
    market::{BookLevel, OrderBookUpdate},
    primitives::{Price, Qty},
};

/// The state of a local order book after applying snapshots and deltas.
#[derive(Debug, Clone)]
pub struct LocalOrderBook {
    /// Bids sorted ascending by price - best bid is the *last* entry.
    bids: BTreeMap<Price, Qty>,
    /// Asks sorted ascending by price - best ask is the *first* entry.
    asks: BTreeMap<Price, Qty>,
    /// The sequence number of the last update we applied.
    last_seq: u64,
    /// Whether we have a valid snapshot to apply deltas against.
    /// False at startup and after a sequence gap.
    initialised: bool,
}

impl LocalOrderBook {
    pub fn new() -> Self {
        Self {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            last_seq: 0,
            initialised: false,
        }
    }

    /// Apply a snapshot or delta update from the exchange.
    ///
    /// Returns `true` if the update was applied successfully.
    /// Returns `false` if the update was skipped (gap detected, pre-snapshot delta).
    pub fn apply(&mut self, update: &OrderBookUpdate) -> bool {
        if update.is_snapshot {
            self.apply_snapshot(update);
            return true;
        }

        // Delta - only valid if we have a snapshot and sequence is gapless
        if !self.initialised {
            debug!(seq = update.sequence, "Skipping delta - no snapshot yet");
            return false;
        }

        // Check for sequence gap
        // Bybit's update IDs increment by 1 for each message
        if update.sequence != self.last_seq + 1 {
            warn!(
                expected = self.last_seq + 1,
                got = update.sequence,
                "Sequence gap detected - book is stale, waiting for re-sync"
            );
            self.initialised = false;
            self.bids.clear();
            self.asks.clear();
            return false;
        }

        self.apply_delta(update);
        true
    }

    /// Replace the entire book with snapshot data.
    fn apply_snapshot(&mut self, update: &OrderBookUpdate) {
        self.bids.clear();
        self.asks.clear();

        for level in &update.bids {
            if !level.qty.is_zero() {
                self.bids.insert(level.price, level.qty);
            }
        }
        for level in &update.asks {
            if !level.qty.is_zero() {
                self.asks.insert(level.price, level.qty);
            }
        }

        self.last_seq = update.sequence;
        self.initialised = true;

        debug!(
            seq = update.sequence,
            bids = self.bids.len(),
            asks = self.asks.len(),
            "Snapshot applied"
        );
    }

    /// Apply an incremental delta - insert, update, or delete levels.
    fn apply_delta(&mut self, update: &OrderBookUpdate) {
        // Apply bid changes
        for level in &update.bids {
            apply_level(&mut self.bids, level);
        }
        // Apply ask changes
        for level in &update.asks {
            apply_level(&mut self.asks, level);
        }

        self.last_seq = update.sequence;
    }

    //  Queries

    /// Best bid price - highest price a buyer is willing to pay.
    /// `None` if the book is empty or uninitialised.
    pub fn best_bid(&self) -> Option<Price> {
        self.bids.keys().next_back().copied()
    }

    /// Best ask price - lowest price a seller is willing to accept.
    /// `None` if the book is empty or uninitialised.
    pub fn best_ask(&self) -> Option<Price> {
        self.asks.keys().next().copied()
    }

    /// The bid-ask spread = best_ask - best_bid.
    /// `None` if either side is empty.
    pub fn spread(&self) -> Option<rust_decimal::Decimal> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => Some(ask.inner() - bid.inner()),
            _ => None,
        }
    }

    /// Mid price = (best_bid + best_ask) / 2.
    pub fn mid_price(&self) -> Option<rust_decimal::Decimal> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => {
                Some((bid.inner() + ask.inner()) / rust_decimal::Decimal::TWO)
            }
            _ => None,
        }
    }

    /// Total quantity available within `depth` levels on the bid side.
    pub fn bid_depth(&self, levels: usize) -> Qty {
        self.bids
            .values()
            .rev()
            .take(levels)
            .fold(Qty::zero(), |acc, &q| acc + q)
    }

    /// Total quantity available within `depth` levels on the ask side.
    pub fn ask_depth(&self, levels: usize) -> Qty {
        self.asks
            .values()
            .take(levels)
            .fold(Qty::zero(), |acc, &q| acc + q)
    }

    pub fn is_initialised(&self) -> bool {
        self.initialised
    }
    pub fn last_seq(&self) -> u64 {
        self.last_seq
    }
}

impl Default for LocalOrderBook {
    fn default() -> Self {
        Self::new()
    }
}

/// Apply one `BookLevel` to a side of the book.
/// qty == 0 means delete the level; otherwise insert/replace.
fn apply_level(side: &mut BTreeMap<Price, Qty>, level: &BookLevel) {
    if level.qty.is_zero() {
        side.remove(&level.price);
    } else {
        side.insert(level.price, level.qty);
    }
}
