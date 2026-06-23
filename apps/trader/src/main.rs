//! Phase 0 — proves the workspace compiles and domain types work.
//! Nothing reaches the network yet.

mod telemetry;

use hft_core::instrument::{AssetPair, Exchange, Instrument};
use hft_core::market::Side;
use hft_core::order::{OrderIntent, OrderState};
use hft_core::primitives::{Price, Qty};
use rust_decimal_macros::dec;
use tracing::{info, warn};


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    telemetry::init();

    info!(version = env!("CARGO_PKG_VERSION"), "hft-crypto — Phase 0");

    // Domain types
    let instrument = Instrument::new(Exchange::Binance, AssetPair::new("BTC", "USDT"));
    info!(instrument = %instrument, "Instrument OK");

    let price    = Price::new(dec!(65_000.50))?;
    let qty      = Qty::new(dec!(0.001))?;
    let notional = price * qty;
    info!(price = %price, qty = %qty, notional = %notional, "Price × Qty = Notional");

    // Order lifecycle
    let intent = OrderIntent::limit(instrument, Side::Buy, qty, price, "phase0-001");
    let state  = OrderState::new(intent);
    info!(id = %state.intent.client_order_id, status = ?state.status, "Order created");

    // Validation catches bad values at construction
    match Price::new(dec!(-1)) {
        Err(e) => warn!("Correctly rejected: {e}"),
        Ok(_)  => panic!("should have rejected negative price"),
    }

    info!("Phase 0 complete ✓ - next: Phase 1 (market data ingestion)");
    Ok(())
}