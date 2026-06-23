//! Raw Bybit V5 WebSocket wire types.
//!
//! These structs exist solely to deserialise Bybit's JSON.
#![allow(dead_code)]
//! They are NEVER used outside this module — the parser converts them
//! into clean `MarketEvent` domain types immediately.
//!
//! Field names match Bybit's API exactly so serde needs no renaming.
//! When Bybit changes their API, only this file and the parser need updating.

use serde::Deserialize;

/// Top-level envelope — Bybit wraps everything in topic/type/data.
/// We use an untagged enum so serde picks the right variant by field shape.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum BybitWsMessage {
    Trade(TradeEnvelope),
    OrderBook(OrderBookEnvelope),
}

// Trades

#[derive(Debug, Deserialize)]
pub struct TradeEnvelope {
    pub topic: String, // "publicTrade.BTCUSDT"
    pub ts: u64,       // message timestamp ms
    pub data: Vec<RawTrade>,
}

/// One trade as Bybit sends it.
/// Field names are Bybit's single-letter abbreviations.
#[derive(Debug, Deserialize)]
pub struct RawTrade {
    /// Trade timestamp in milliseconds
    #[serde(rename = "T")]
    pub ts_ms: u64,
    /// Symbol e.g. "BTCUSDT"
    #[serde(rename = "s")]
    pub symbol: String,
    /// Side: "Buy" or "Sell" (the aggressor)
    #[serde(rename = "S")]
    pub side: String,
    /// Quantity as string e.g. "0.001"
    #[serde(rename = "v")]
    pub qty: String,
    /// Price as string e.g. "16578.50"
    #[serde(rename = "p")]
    pub price: String,
    /// Trade ID
    #[serde(rename = "i")]
    pub trade_id: String,
    /// Is block trade
    #[serde(rename = "BT")]
    pub is_block_trade: bool,
}

//  Order Book

#[derive(Debug, Deserialize)]
pub struct OrderBookEnvelope {
    pub topic: String, // "orderbook.50.BTCUSDT"
    pub ts: u64,       // message timestamp ms
    #[serde(rename = "type")]
    pub msg_type: String, // "snapshot" or "delta"
    pub data: OrderBookData,
    pub cts: u64, // confirmed timestamp ms
}

#[derive(Debug, Deserialize)]
pub struct OrderBookData {
    /// Symbol
    pub s: String,
    /// Bids: array of ["price", "qty"] string pairs
    pub b: Vec<[String; 2]>,
    /// Asks: array of ["price", "qty"] string pairs
    pub a: Vec<[String; 2]>,
    /// Update ID — must be monotonically increasing, use for gap detection
    pub u: u64,
    /// Global sequence number
    pub seq: u64,
}
