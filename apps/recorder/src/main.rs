//! Recorder — Phase 1.
//!
//! Connects to Bybit testnet, maintains a local order book,
//! and prints market state on every update.
//! Phase 2 will persist events to disk instead of stdout.

use exchanges::bybit::{self, orderbook::LocalOrderBook};
use hft_core::instrument::{AssetPair, Exchange, Instrument};
use hft_core::market::MarketEventKind;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .pretty()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_file(true)
        .with_line_number(true)
        .init();

    let instrument = Instrument::new(Exchange::Bybit, AssetPair::new("BTC", "USDT"));
    info!(instrument = %instrument, "Recorder starting — Phase 1");

    let mut rx   = bybit::spawn(instrument, 1024);
    let mut book = LocalOrderBook::new();

    while let Some(event) = rx.recv().await {
        match &event.kind {
            MarketEventKind::Trade(t) => {
                info!(
                    side       = %t.aggressor_side,
                    price      = %t.price,
                    qty        = %t.qty,
                    latency_ms = (t.local_ts - t.exchange_ts).whole_milliseconds(),
                    mid_price  = ?book.mid_price(),
                    "TRADE"
                );
            }

            MarketEventKind::OrderBookUpdate(ob) => {
                let applied = book.apply(ob);
                if !applied { continue; }

                info!(
                    seq       = book.last_seq(),
                    best_bid  = ?book.best_bid(),
                    best_ask  = ?book.best_ask(),
                    spread    = ?book.spread(),
                    bid_depth = %book.bid_depth(5),
                    ask_depth = %book.ask_depth(5),
                    "BOOK"
                );
            }
        }
    }

    Ok(())
}