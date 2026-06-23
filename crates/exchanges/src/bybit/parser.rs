//! Bybit wire → `MarketEvent` domain type conversion.
//!
//! Every function here takes a raw Bybit type and returns a `MarketEvent`
//! or an error. If parsing fails (bad decimal string, unknown side),
//! we return an error and the caller decides whether to skip or crash.
//!
//! ## Why parse strings to Decimal here (not in types.rs)?
//!
//! Bybit sends prices and quantities as JSON strings ("62013.9"), not
//! numbers. This is intentional on their side — JSON numbers lose
//! precision for large decimals. We parse them into `rust_decimal::Decimal`
//! here, at the boundary, so the rest of the system never sees strings.

use std::str::FromStr;

use rust_decimal::Decimal;
use time::OffsetDateTime;

use hft_core::{
    instrument::Instrument,
    market::{BookLevel, MarketEvent, MarketEventKind, OrderBookUpdate, Side, Trade},
    primitives::{Price, Qty},
};

use super::types::{OrderBookEnvelope, RawTrade};

/// Parse error — something in the wire data was malformed.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("invalid decimal string '{0}': {1}")]
    InvalidDecimal(String, rust_decimal::Error),
    #[error("invalid price value: {0}")]
    InvalidPrice(#[from] hft_core::error::CoreError),
    #[error("unknown side '{0}'")]
    UnknownSide(String),
}

// Trades

pub fn parse_trade(instrument: Instrument, raw: RawTrade) -> Result<MarketEvent, ParseError> {
    let price = parse_price(&raw.price)?;
    let qty = parse_qty(&raw.qty)?;
    let side = parse_side(&raw.side)?;

    // Bybit gives ms timestamps — convert to OffsetDateTime
    let exchange_ts = ms_to_datetime(raw.ts_ms);
    let local_ts = OffsetDateTime::now_utc();

    let trade = Trade {
        trade_id: Some(raw.trade_id),
        price,
        qty,
        aggressor_side: side,
        exchange_ts,
        local_ts,
    };

    Ok(MarketEvent::new(instrument, MarketEventKind::Trade(trade)))
}

// Order Book

pub fn parse_orderbook(
    instrument: Instrument,
    envelope: OrderBookEnvelope,
) -> Result<MarketEvent, ParseError> {
    let is_snapshot = envelope.msg_type == "snapshot";

    let bids = parse_levels(&envelope.data.b)?;
    let asks = parse_levels(&envelope.data.a)?;

    let exchange_ts = ms_to_datetime(envelope.ts);
    let local_ts = OffsetDateTime::now_utc();

    let update = OrderBookUpdate {
        bids,
        asks,
        is_snapshot,
        sequence: envelope.data.u, // u = update ID for gap detection
        exchange_ts,
        local_ts,
    };

    Ok(MarketEvent::new(
        instrument,
        MarketEventKind::OrderBookUpdate(update),
    ))
}

// Helpers

fn parse_levels(raw: &[[String; 2]]) -> Result<Vec<BookLevel>, ParseError> {
    raw.iter()
        .map(|[p, q]| {
            Ok(BookLevel {
                price: parse_price(p)?,
                qty: parse_qty(q)?,
            })
        })
        .collect()
}

fn parse_price(s: &str) -> Result<Price, ParseError> {
    let d = Decimal::from_str(s).map_err(|e| ParseError::InvalidDecimal(s.to_string(), e))?;
    Ok(Price::new(d)?)
}

fn parse_qty(s: &str) -> Result<Qty, ParseError> {
    let d = Decimal::from_str(s).map_err(|e| ParseError::InvalidDecimal(s.to_string(), e))?;
    // qty=0 is valid (level deletion) so we use Qty::new not TryFrom
    Ok(Qty::new(d)?)
}

fn parse_side(s: &str) -> Result<Side, ParseError> {
    match s {
        "Buy" => Ok(Side::Buy),
        "Sell" => Ok(Side::Sell),
        other => Err(ParseError::UnknownSide(other.to_string())),
    }
}

fn ms_to_datetime(ms: u64) -> OffsetDateTime {
    // OffsetDateTime::from_unix_timestamp takes seconds + nanoseconds
    let secs = (ms / 1000) as i64;
    let nanos = ((ms % 1000) * 1_000_000) as u32;
    OffsetDateTime::from_unix_timestamp(secs).unwrap_or(OffsetDateTime::UNIX_EPOCH)
        + time::Duration::nanoseconds(nanos as i64)
}
