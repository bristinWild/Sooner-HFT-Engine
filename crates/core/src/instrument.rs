//! What you're trading (`AssetPair`) and where (`Exchange`) = `Instrument`.
//!
//! Using an enum for `Exchange` (not a String) means every match is
//! exhaustive — the compiler forces you to handle a new venue everywhere
//! when you add it. No silent "forgotten case" bugs.

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Exchange {
    Binance,
    Bybit,
    Okx,
}

impl fmt::Display for Exchange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Exchange::Binance => write!(f, "binance"),
            Exchange::Bybit => write!(f, "bybit"),
            Exchange::Okx => write!(f, "okx"),
        }
    }
}

/// A trading pair — base asset / quote currency.
/// base = what you buy/sell (BTC), quote = pricing currency (USDT).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AssetPair {
    pub base: String,
    pub quote: String,
}

impl AssetPair {
    pub fn new(base: impl Into<String>, quote: impl Into<String>) -> Self {
        Self {
            base: base.into().to_uppercase(),
            quote: quote.into().to_uppercase(),
        }
    }

    /// Bare symbol used in most exchange APIs: `"BTCUSDT"`
    pub fn symbol(&self) -> String {
        format!("{}{}", self.base, self.quote)
    }
}

impl fmt::Display for AssetPair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.base, self.quote)
    }
}

/// The combination of *what* and *where*. The primary routing key.
///
/// Two instruments with the same pair on different exchanges are different
/// things — different prices, fees, and execution rules.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Instrument {
    pub exchange: Exchange,
    pub pair: AssetPair,
}

impl Instrument {
    pub fn new(exchange: Exchange, pair: AssetPair) -> Self {
        Self { exchange, pair }
    }
}

impl fmt::Display for Instrument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.exchange, self.pair)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pair_normalises_to_uppercase() {
        let p = AssetPair::new("btc", "usdt");
        assert_eq!(p.base, "BTC");
    }

    #[test]
    fn instrument_display() {
        let i = Instrument::new(Exchange::Binance, AssetPair::new("BTC", "USDT"));
        assert_eq!(i.to_string(), "binance:BTC/USDT");
    }

    #[test]
    fn same_pair_different_exchange_not_equal() {
        let a = Instrument::new(Exchange::Binance, AssetPair::new("BTC", "USDT"));
        let b = Instrument::new(Exchange::Bybit, AssetPair::new("BTC", "USDT"));
        assert_ne!(a, b);
    }
}
