# Phase 1 - Market Data Ingestion & Order Book

**Goal:** Connect to Bybit testnet, stream live trades and order book updates,
parse them into domain types, and maintain a local order book.

**Done:** `cargo run -p recorder` streams live data with a stable connection,
correct sequence validation, and meaningful market state on every update.

---

## What we built

### Connector (`crates/exchanges/src/bybit/`)

Three files with strictly separated responsibilities:

- `types.rs` - raw wire structs mirroring Bybit's exact JSON. No logic.
- `parser.rs` - converts wire types to `MarketEvent`. The only place
  string prices are parsed to `Decimal`. If Bybit changes their API,
  only these two files need updating.
- `mod.rs` - async WebSocket loop, subscription management, reconnect,
  and heartbeat. Emits `MarketEvent`s on a bounded `mpsc` channel.
- `orderbook.rs` - `LocalOrderBook` backed by two `BTreeMap<Price, Qty>`.

### Key design decisions

**Bounded channel (buffer: 1024).** If the consumer falls behind, the
connector blocks rather than growing memory unboundedly. This is
backpressure - the system slows down instead of silently dropping events
or running out of RAM.

**Strings → Decimal at the boundary.** Bybit sends `"62013.9"` not
`62013.9`. We parse to `Decimal` in `parser.rs` immediately. Nothing
downstream ever sees a price as a string or a float.

**`BTreeMap` for the order book.** Keys (prices) stay sorted for free.
`best_bid()` = `keys().next_back()`, `best_ask()` = `keys().next()`.
Both are O(log n). For 50 levels this is faster than a `Vec` with sorting.

**Sequence gap detection.** If `delta.seq != last_seq + 1`, the book is
stale. We clear it and wait for the next snapshot rather than applying
deltas against incorrect state. Getting this wrong produces a corrupted
book that silently misprices the market.

**Heartbeat every 20s.** Bybit closes idle connections after ~60s.
We send `{"op":"ping"}` every 20s inside a `tokio::select!` loop
alongside the message handler.

### Latency observed

On testnet over a home connection: 22–90ms exchange → local.
On mainnet with a cloud VM in the same region as the exchange
(e.g. AWS Tokyo for Bybit): expect 1–5ms.

---

## What to watch out for

**The snapshot/delta race.** There's a window between subscribing and
receiving the snapshot where deltas can arrive. We handle this by
discarding deltas until `initialised = true`. Some exchanges require
you to buffer deltas and replay them after the snapshot - Bybit's
testnet doesn't seem to require this, but watch for it on mainnet.

**Testnet spreads are unrealistic.** 224 USDT spread and heavily
imbalanced depth are artifacts of low testnet activity. Strategy
signals built on testnet data will not reflect mainnet conditions.

---

## Next: Phase 2 - Backtesting Engine

Phase 2 writes market events to disk (the `recorder` binary's real job)
and builds a replay engine that drives the same `LocalOrderBook` and
strategy code from recorded data. The goal: test strategies offline
before they touch live markets.