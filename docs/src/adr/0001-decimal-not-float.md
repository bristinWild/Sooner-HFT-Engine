# ADR-0001: Use `rust_decimal::Decimal` for all financial values

**Date:** Phase 0  
**Status:** Accepted

---

## Context

Every financial value - prices, quantities, fees, PnL - must be some
numeric type. The two main candidates in Rust are `f64` and `Decimal`.

## Decision

Use `Decimal` for all financial values. Never use `f64` for prices,
quantities, notionals, fees, or balances.

## Consequences

### Why not f64?

`f64` cannot represent many decimal fractions exactly:

```text
0.1 + 0.2 = 0.30000000000000004  (f64)
0.1 + 0.2 = 0.3                  (Decimal) ✓
```

In a trading system this causes wrong PnL, wrong order sizes, and
order rejections from the exchange. Real money is lost to these bugs.

### Why Decimal?

`rust_decimal::Decimal` stores values as `(coefficient, base-10 scale)`.
`123.45` is stored as `(12345, 2)`. This matches how exchanges represent
numbers - no precision loss in conversions.

### Trade-offs

`Decimal` is ~5–20x slower than `f64` for arithmetic. Acceptable because
network IO is the bottleneck in a trading system, not math. If a specific
hot path is measurably slow, we can use `i64` fixed-point there,
explicitly documented as a performance optimisation.

## Alternatives considered

- **`f64`**: rejected - precision bugs are unacceptable in financial code.
- **`i64` fixed-point**: viable hot-path optimisation, not a default.
- **`bigdecimal`**: worse `serde` support than `rust_decimal`.