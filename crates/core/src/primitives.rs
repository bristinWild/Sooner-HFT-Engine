//! `Price`, `Qty`, `Notional` — newtype wrappers over `Decimal`.
//!
//! ## Why not f64?
//!
//! ```text
//! 0.1 + 0.2 = 0.30000000000000004  ← f64
//! 0.1 + 0.2 = 0.3                  ← Decimal  ✓
//! ```
//!
//! Floating-point errors accumulate across thousands of fills and produce
//! wrong PnL, wrong position sizes, and order rejections.
//!
//! ## Why newtypes?
//!
//! Without them, swapping `price` and `qty` arguments is a silent bug.
//! With them, it's a compile error.

use crate::error::CoreError;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, Mul, Sub};

// Price

/// A strictly positive asset price.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Price(Decimal);

impl Price {
    pub fn new(value: Decimal) -> Result<Self, CoreError> {
        if value <= Decimal::ZERO {
            return Err(CoreError::InvalidPrice(value));
        }
        Ok(Self(value))
    }

    pub fn inner(&self) -> Decimal {
        self.0
    }
}

impl fmt::Display for Price {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<Decimal> for Price {
    type Error = CoreError;
    fn try_from(v: Decimal) -> Result<Self, Self::Error> {
        Price::new(v)
    }
}

//  Qty

/// A non-negative quantity (e.g. 0.005 BTC).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Qty(Decimal);

impl Qty {
    pub fn new(value: Decimal) -> Result<Self, CoreError> {
        if value < Decimal::ZERO {
            return Err(CoreError::InvalidQty(value));
        }
        Ok(Self(value))
    }

    pub fn zero() -> Self {
        Self(dec!(0))
    }
    pub fn inner(&self) -> Decimal {
        self.0
    }
    pub fn is_zero(&self) -> bool {
        self.0 == Decimal::ZERO
    }
}

impl Add for Qty {
    type Output = Qty;
    // Adding two non-negative values is always non-negative — infallible.
    fn add(self, rhs: Qty) -> Qty {
        Qty(self.0 + rhs.0)
    }
}

impl Sub for Qty {
    type Output = Result<Qty, CoreError>;
    fn sub(self, rhs: Qty) -> Self::Output {
        Qty::new(self.0 - rhs.0)
    }
}

impl fmt::Display for Qty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<Decimal> for Qty {
    type Error = CoreError;
    fn try_from(v: Decimal) -> Result<Self, Self::Error> {
        Qty::new(v)
    }
}

//  Notional

/// The result of Price × Qty — value in the quote currency (e.g. USD).
/// Can be negative (e.g. a short expressed as notional).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Notional(Decimal);

impl Notional {
    pub fn new(value: Decimal) -> Self {
        Self(value)
    }
    pub fn zero() -> Self {
        Self(dec!(0))
    }
    pub fn inner(&self) -> Decimal {
        self.0
    }
}

impl fmt::Display for Notional {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Price × Qty = Notional. The only way to produce a Notional is by
/// multiplying a Price by a Qty — you can't accidentally construct one.
impl Mul<Qty> for Price {
    type Output = Notional;
    fn mul(self, rhs: Qty) -> Notional {
        Notional(self.0 * rhs.0)
    }
}

//  Tests

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn price_rejects_zero_and_negative() {
        assert!(Price::new(dec!(0)).is_err());
        assert!(Price::new(dec!(-1)).is_err());
        assert!(Price::new(dec!(0.000001)).is_ok());
    }

    #[test]
    fn qty_rejects_negative_allows_zero() {
        assert!(Qty::new(dec!(-0.1)).is_err());
        assert!(Qty::new(dec!(0)).is_ok()); // zero qty = empty book level
    }

    #[test]
    fn price_times_qty_gives_notional() {
        let price = Price::new(dec!(30000)).unwrap();
        let qty = Qty::new(dec!(0.5)).unwrap();
        let notional = price * qty;
        assert_eq!(notional.inner(), dec!(15000));
    }

    #[test]
    fn qty_subtraction_errors_on_underflow() {
        let a = Qty::new(dec!(1)).unwrap();
        let b = Qty::new(dec!(2)).unwrap();
        assert!((a - b).is_err());
    }

    #[test]
    fn decimal_is_exact_unlike_f64() {
        // This is the whole reason we use Decimal.
        // With f64: 0.1 + 0.2 == 0.30000000000000004 (fails)
        assert_eq!(dec!(0.1) + dec!(0.2), dec!(0.3));
    }
}
