use rust_decimal::Decimal;
use thiserror::Error;

/// The single error type for the `core` crate.
///
/// Each crate in the workspace defines its own error type.
/// This scopes error variants so callers know which layer failed.
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("invalid price: {0} — must be strictly positive")]
    InvalidPrice(Decimal),

    #[error("invalid quantity: {0} — must be non-negative")]
    InvalidQty(Decimal),

    #[error("quantity underflow: result would be negative")]
    QtyUnderflow,
}
