//! The order lifecycle: Intent → Pending → Open → Filled/Cancelled/Rejected.
//!
//! `OrderIntent` = what the strategy *wants*.
//! `OrderState`  = what the OMS *knows is true* on the exchange.
//!
//! Keeping them separate is intentional: a strategy cannot assume its intent
//! was executed. It must observe `OrderState` changes to know reality.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::error::CoreError;
use crate::instrument::Instrument;
use crate::market::Side;
use crate::primitives::{Notional, Price, Qty};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderType {
    /// Immediate execution at best available price. Always a taker.
    Market,
    /// Rest on the book. Usually a maker.
    Limit,
    /// Limit order that is cancelled if it would match immediately.
    /// Guarantees maker execution. Required for market-making.
    LimitMakerOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeInForce {
    GTC, // Good Till Cancelled
    IOC, // Immediate Or Cancel
    FOK, // Fill Or Kill
}

/// A strategy's request to place an order.
/// The risk manager may reject or resize this before it reaches the exchange.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderIntent {
    pub instrument: Instrument,
    pub side: Side,
    pub order_type: OrderType,
    pub qty: Qty,
    pub limit_price: Option<Price>,
    pub time_in_force: TimeInForce,
    pub client_order_id: String,
}

impl OrderIntent {
    pub fn limit(
        instrument: Instrument,
        side: Side,
        qty: Qty,
        price: Price,
        client_order_id: impl Into<String>,
    ) -> Self {
        Self {
            instrument,
            side,
            order_type: OrderType::Limit,
            qty,
            limit_price: Some(price),
            time_in_force: TimeInForce::GTC,
            client_order_id: client_order_id.into(),
        }
    }

    pub fn market(
        instrument: Instrument,
        side: Side,
        qty: Qty,
        client_order_id: impl Into<String>,
    ) -> Self {
        Self {
            instrument,
            side,
            order_type: OrderType::Market,
            qty,
            limit_price: None,
            time_in_force: TimeInForce::IOC,
            client_order_id: client_order_id.into(),
        }
    }

    pub fn tif(mut self, tif: TimeInForce) -> Self {
        self.time_in_force = tif;
        self
    }
}

/// A (partial) fill reported by the exchange for one of our orders.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fill {
    pub client_order_id: String,
    pub exchange_order_id: String,
    pub instrument: Instrument,
    pub side: Side,
    pub fill_price: Price,
    pub fill_qty: Qty,
    pub notional: Notional,
    pub fee: Notional,
    pub exchange_ts: OffsetDateTime,
    pub local_ts: OffsetDateTime,
}

/// Live status of an order. State machine:
///
/// ```text
/// Pending → Open → PartiallyFilled → Filled   (terminal)
///         ↘ Cancelled                          (terminal)
///         ↘ Rejected { reason }                (terminal)
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderStatus {
    Pending,
    Open,
    PartiallyFilled,
    Filled,
    Cancelled,
    Rejected { reason: String },
}

/// An `OrderIntent` with live exchange state attached. Owned by the OMS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderState {
    pub intent: OrderIntent,
    pub exchange_order_id: Option<String>,
    pub status: OrderStatus,
    pub filled_qty: Qty,
    pub fills: Vec<Fill>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl OrderState {
    pub fn new(intent: OrderIntent) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            intent,
            exchange_order_id: None,
            status: OrderStatus::Pending,
            filled_qty: Qty::zero(),
            fills: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn remaining_qty(&self) -> Result<Qty, CoreError> {
        self.intent.qty - self.filled_qty
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            OrderStatus::Filled | OrderStatus::Cancelled | OrderStatus::Rejected { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instrument::{AssetPair, Exchange};
    use rust_decimal_macros::dec;

    fn btc() -> Instrument {
        Instrument::new(Exchange::Binance, AssetPair::new("BTC", "USDT"))
    }

    #[test]
    fn new_order_state_is_pending_and_not_terminal() {
        let intent = OrderIntent::market(btc(), Side::Buy, Qty::new(dec!(0.01)).unwrap(), "t1");
        let state = OrderState::new(intent);
        assert_eq!(state.status, OrderStatus::Pending);
        assert!(!state.is_terminal());
    }

    #[test]
    fn filled_is_terminal() {
        let intent = OrderIntent::market(btc(), Side::Buy, Qty::new(dec!(0.01)).unwrap(), "t2");
        let mut state = OrderState::new(intent);
        state.status = OrderStatus::Filled;
        assert!(state.is_terminal());
    }

    #[test]
    fn remaining_qty_with_no_fills_equals_total() {
        let qty = Qty::new(dec!(0.1)).unwrap();
        let intent = OrderIntent::market(btc(), Side::Buy, qty, "t3");
        let state = OrderState::new(intent);
        assert_eq!(state.remaining_qty().unwrap().inner(), dec!(0.1));
    }
}
