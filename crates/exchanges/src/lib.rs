//! # exchanges
//!
//! Exchange connectors. Each connector:
//! 1. Opens a WebSocket to the exchange
//! 2. Parses raw wire messages into `MarketEvent`
//! 3. Sends them down a channel to consumers
//!
//! Nothing in this crate places orders — that's `execution` (Phase 4).

pub mod bybit;
