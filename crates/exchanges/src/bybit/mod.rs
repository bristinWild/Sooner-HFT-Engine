//! Bybit V5 public WebSocket connector.
//!
//! Connects to the spot public stream, subscribes to trades and order book,
//! and emits `MarketEvent`s on a tokio channel.
//!
//! ## Reconnection
//!
//! The connector reconnects automatically on disconnect. Bybit sends a
//! ping every 20 seconds; we must reply with pong or the server closes
//! the connection after ~30 seconds.

mod parser;
mod types;
pub mod orderbook;

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{error, info, warn};

use hft_core::instrument::Instrument;
use hft_core::market::MarketEvent;

use self::types::BybitWsMessage;

/// Public WebSocket URL for Bybit testnet spot.
const WS_URL: &str = "wss://stream-testnet.bybit.com/v5/public/spot";

/// How long to wait before reconnecting after a disconnect.
const RECONNECT_DELAY: Duration = Duration::from_secs(2);

/// Errors from the Bybit connector.
#[derive(Debug, thiserror::Error)]
pub enum ConnectorError {
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Channel closed — no receivers")]
    ChannelClosed,
}

/// Spawns the Bybit connector as a background task.
///
/// Returns a channel receiver - pull `MarketEvent`s from it in your
/// strategy or recorder. The connector reconnects automatically.
///
/// # Arguments
/// * `instrument` - the market to subscribe to (must be BTC/USDT for now)
/// * `buffer` - channel buffer size (backpressure: sender blocks when full)
pub fn spawn(
    instrument: Instrument,
    buffer: usize,
) -> mpsc::Receiver<MarketEvent> {
    let (tx, rx) = mpsc::channel(buffer);

    tokio::spawn(async move {
        loop {
            match run(&instrument, tx.clone()).await {
                Ok(()) => {
                    info!("Connector exited cleanly — reconnecting");
                }
                Err(e) => {
                    error!(error = %e, "Connector error — reconnecting in {}s",
                        RECONNECT_DELAY.as_secs());
                }
            }

            // If the receiver is gone, stop reconnecting.
            if tx.is_closed() {
                info!("Channel closed — connector shutting down");
                break;
            }

            tokio::time::sleep(RECONNECT_DELAY).await;
        }
    });

    rx
}

/// Core connection loop - runs until error or clean shutdown.
async fn run(
    instrument: &Instrument,
    tx: mpsc::Sender<MarketEvent>,
) -> Result<(), ConnectorError> {
    info!(url = WS_URL, "Connecting to Bybit testnet");

    let (mut ws, _) = connect_async(WS_URL).await?;
    info!("WebSocket connected");

    let symbol = instrument.pair.symbol();
    let sub = serde_json::json!({
        "op": "subscribe",
        "args": [
            format!("publicTrade.{symbol}"),
            format!("orderbook.50.{symbol}"),
        ]
    });
    ws.send(Message::Text(sub.to_string().into())).await?;
    info!(symbol, "Subscribed to publicTrade + orderbook.50");

    // Bybit requires a ping every 20s or it closes the connection.
    let mut heartbeat = tokio::time::interval(Duration::from_secs(20));
    heartbeat.tick().await; // consume the immediate first tick

    loop {
        tokio::select! {
            msg = ws.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        handle_text(&text, instrument, &tx).await?;
                    }
                    Some(Ok(Message::Ping(payload))) => {
                        ws.send(Message::Pong(payload)).await?;
                    }
                    Some(Ok(Message::Close(frame))) => {
                        warn!(frame = ?frame, "Server closed connection");
                        break;
                    }
                    Some(Ok(_)) => {} // Pong, Binary, Frame — ignore
                    Some(Err(e)) => return Err(ConnectorError::WebSocket(e)),
                    None => break, // stream ended
                }
            }
            _ = heartbeat.tick() => {
                // Bybit's keepalive — send ping, expect pong back
                let ping = serde_json::json!({"op": "ping"});
                ws.send(Message::Text(ping.to_string().into())).await?;
                tracing::debug!("Sent heartbeat ping");
            }
        }
    }

    Ok(())
}
/// Parse one text message and forward any resulting MarketEvent.
async fn handle_text(
    text: &str,
    instrument: &Instrument,
    tx: &mpsc::Sender<MarketEvent>,
) -> Result<(), ConnectorError> {
    // Try to deserialize as a known message type.
    let msg: BybitWsMessage = match serde_json::from_str(text) {
        Ok(m)  => m,
        Err(e) => {
            // Don't crash on unknown messages (subscription acks, heartbeats).
            // Log at debug so we can inspect them without noise.
            tracing::debug!(error = %e, raw = %text, "Unrecognised WS message — skipping");
            return Ok(());
        }
    };

    match msg {
        BybitWsMessage::Trade(envelope) => {
            for raw_trade in envelope.data {
                match parser::parse_trade(instrument.clone(), raw_trade) {
                    Ok(event) => {
                        if tx.send(event).await.is_err() {
                            return Err(ConnectorError::ChannelClosed);
                        }
                    }
                    Err(e) => {
                        warn!(error = %e, "Failed to parse trade — skipping");
                    }
                }
            }
        }
        BybitWsMessage::OrderBook(envelope) => {
            match parser::parse_orderbook(instrument.clone(), envelope) {
                Ok(event) => {
                    if tx.send(event).await.is_err() {
                        return Err(ConnectorError::ChannelClosed);
                    }
                }
                Err(e) => {
                    warn!(error = %e, "Failed to parse order book — skipping");
                }
            }
        }
    }

    Ok(())
}