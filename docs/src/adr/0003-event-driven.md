# ADR-0003: Event-driven architecture with typed channel events

**Date:** Phase 0  
**Status:** Accepted

---

## Context

The system handles multiple concurrent data streams, reacts to market
events with low latency, and processes order lifecycle updates - all
while maintaining consistent internal state.

## Decision

Event-driven architecture where components communicate by passing typed
events through bounded async channels (`tokio::sync::mpsc`).

The central type is `MarketEvent`, produced by exchange connectors and
consumed by order book builders, strategies, and recorders.

## Consequences

### Why event-driven?

**Decoupling:** the connector doesn't know who consumes its events.
Adding a new consumer requires no changes to the connector.

**Testability:** a strategy can be tested by sending it synthetic
`MarketEvent`s - no network connection needed.

**Backtest reuse:** the backtester replays recorded `MarketEvent`s
through the exact same strategy and OMS code as live trading. If the
backtest and live paths diverged, the backtest would be meaningless.

**Natural fit for async Rust:** tokio channels map directly onto
this model.

### Bounded channels

We use bounded channels everywhere. If a consumer can't keep up, the
channel fills and the producer blocks - backpressure rather than
unbounded memory growth.

### Trade-offs

More complex than request/response - you can't call `get_price()` and
get an answer. You subscribe to a stream and react. This is the correct
mental model for markets (push, not pull) but takes adjustment.