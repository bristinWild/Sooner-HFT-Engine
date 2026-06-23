# Rust Crypto Algo-Trading Engine - Roadmap & Architecture

> A learning-first, event-driven HFT system in Rust. Built phase by phase, with
> documentation written alongside the code so any developer can read the repo and
> understand *why* each piece exists, not just *what* it does.

---

## 0. Read this first - goals & honest expectations

**Primary goal:** learn systems programming, market microstructure, and event-driven
architecture in Rust by building a real trading engine end to end.

**Secondary goal:** run modest strategies on your own crypto capital - *after* the
system has earned trust through backtesting and paper trading.

**What this project is NOT:** a guaranteed money machine, and not nanosecond HFT.
Retail cannot win the pure latency race (colocation, FPGAs, direct feeds belong to
professional market makers). Our edge, if any, comes from *correctness, discipline,
and finding small structural inefficiencies* - not from being the fastest on the wire.

**The non-negotiable safety ordering** (never skip a step):

```
backtest on historical data  →  paper-trade on exchange TESTNET
   →  live with TINY size + hard risk limits  →  scale only if it survives
```

Most strategies die at the backtest or paper stage. That is the system working,
not failing - it is saving you real money.

---

## 1. Tech stack

| Concern              | Crate(s)                                  | Notes |
|----------------------|-------------------------------------------|-------|
| Async runtime        | `tokio`                                   | The backbone. |
| WebSocket            | `tokio-tungstenite`                       | Real-time market & account streams. |
| HTTP/REST            | `reqwest`                                 | Snapshots, order placement, historical. |
| Serialization        | `serde`, `serde_json`, `simd-json`        | `simd-json` for hot-path parsing. |
| **Money/prices**     | `rust_decimal` (+ `rust_decimal_macros`)  | **Never use `f64` for prices or balances.** |
| Time                 | `time` or `chrono`                        | Use UTC, nanosecond precision where possible. |
| Channels             | `tokio::sync`, `flume`, `crossbeam`       | Bounded channels for backpressure. |
| Logging/tracing      | `tracing`, `tracing-subscriber`           | Structured logs from day one. |
| Errors               | `thiserror` (libs), `anyhow` (binaries)   | |
| Config               | `figment` or `config`                     | Layered: file + env + secrets. |
| Storage              | `sqlx` + Postgres/TimescaleDB, or Parquet via `arrow` | Time-series of ticks & fills. |
| Metrics              | `metrics` + `metrics-exporter-prometheus` | Feeds Grafana. |
| Benchmarks           | `criterion`                               | Prove your hot paths are fast. |
| Stats                | `statrs`, `ndarray`                       | Sharpe, drawdown, signal math. |

**Reference framework - `barter-rs`** (actively maintained, MIT). Two ways to use it:
1. **As a learning mirror.** Read `barter`, `barter-data`, `barter-execution` to see
   how professionals structure an `Engine`, `Strategy`, and `RiskManager`. Borrow the
   *patterns*, write your own implementation. Best for learning.
2. **As scaffolding.** Build your strategy on top of its `Engine`/`MarketStream` so you
   can reach paper trading faster. Best if you care more about the strategy than the plumbing.

Recommendation for *your* goal (learning + docs): build the core yourself for Phases 1–3,
then compare against `barter-rs` and cherry-pick. You'll learn 10x more.

---

## 2. Repository layout (Cargo workspace)

```
hft-crypto/
├── Cargo.toml                # [workspace]
├── README.md                 # Quick start + link to docs book
├── docs/                     # mdBook - the living documentation
│   ├── book.toml
│   └── src/
│       ├── SUMMARY.md
│       ├── architecture/     # System design, data flow diagrams
│       ├── phases/           # One design doc per phase (the "why")
│       └── adr/              # Architecture Decision Records
├── crates/
│   ├── core/                 # Domain types: Instrument, Price, Side, Order, Event
│   ├── exchanges/            # Per-exchange connectors + a common trait
│   ├── marketdata/           # Ingestion + order book reconstruction
│   ├── backtest/             # Replay + simulated fills + PnL accounting
│   ├── strategy/             # Strategy trait + concrete strategies
│   ├── execution/            # OMS (order lifecycle) + risk manager
│   └── persistence/          # Recording & loading market/trade data
└── apps/
    ├── recorder/             # Binary: record live data to disk
    ├── backtester/           # Binary: run a strategy over recorded data
    └── trader/               # Binary: live/paper trading loop
```

`core` depends on nothing; everything depends on `core`. Keep that arrow pointing one way.

---

## 3. Documentation discipline (your stated priority)

You want a repo a developer can *walk through*. Three layers of docs, maintained as you go:

1. **`rustdoc` (`///`)** - on every public type and function. `cargo doc --open`.
2. **mdBook (`docs/`)** - the narrative. Per phase: the problem, the design, the
   trade-offs, what you'd do differently. This is what makes the repo teachable.
3. **ADRs (`docs/src/adr/`)** - short records of *decisions*: "Why `rust_decimal` not
   `f64`", "Why bounded channels", "Why we replay instead of live-test first." Use the
   classic format: Context → Decision → Consequences. One file per decision, numbered.

**Rule:** a phase isn't "done" until its mdBook chapter and any new ADRs are written.
Code without the doc is an unfinished phase.

---

## 4. The phases

Each phase ends with a working binary or test you can run, plus its docs.

### Phase 0 - Foundations
**Build:** workspace, CI (fmt + clippy + test), `tracing` setup, config loading, and the
`core` domain model - `Instrument`, `Price`/`Qty` (newtypes over `Decimal`), `Side`,
`OrderType`, and the central `MarketEvent` / `OrderEvent` enums.
**Done when:** `cargo test`, `cargo clippy`, and CI are green; domain types compile and
are documented.
**Docs:** architecture overview, ADR-0001 (decimal vs float), ADR-0002 (workspace layout).

### Phase 1 - Market data ingestion & order book
**Build:** connect to ONE exchange's public WebSocket (start with a testnet, e.g. Binance
or Bybit). Parse trades and L2 order-book diffs. Reconstruct a local order book from
snapshot + incremental updates, validating sequence numbers and re-syncing on gaps.
Record raw messages to disk (`persistence`).
**Done when:** your local book matches the exchange's, and you can record a clean session.
**Watch out for:** sequence gaps, out-of-order messages, snapshot/diff race conditions -
the classic order-book bugs. Document your re-sync logic carefully.
**Docs:** exchange protocol notes, order-book reconstruction algorithm, the data model.

### Phase 2 - Backtesting & simulation engine
**Build:** an event loop that replays recorded data through the same interfaces live
trading will use. A *realistic* fill simulator: model latency, maker/taker fees, and
slippage (don't assume you fill at mid). PnL accounting and metrics (Sharpe, Sortino,
max drawdown, win rate, turnover).
**Done when:** you can replay a session and get an honest equity curve for a dummy strategy.
**Watch out for:** look-ahead bias and over-optimistic fills - the two things that make
a backtest lie to you. **This is the most important phase for protecting your money.**
**Docs:** backtest methodology, fill-model assumptions, list of biases you guard against.

### Phase 3 - Strategy framework
**Build:** a `Strategy` trait (`on_event(&mut self, &MarketEvent) -> Vec<OrderIntent>`).
Implement 1–2 baseline strategies - e.g. a simple market maker (quote around mid with an
inventory skew) and/or a cross-exchange spread monitor. Make parameters config-driven.
**Done when:** strategies run inside the backtester and produce interpretable results.
**Docs:** the strategy interface, and a write-up per strategy (hypothesis, logic, when it
should and shouldn't work).

### Phase 4 - Execution (OMS) + risk
**Build:** an Order Management System that places/cancels/amends orders via REST/WS, tracks
each order's lifecycle as a state machine, reconciles fills, and respects exchange rate
limits (with retries + idempotency). A **RiskManager** that every order passes through:
max position, max order size, max daily loss, fat-finger checks, and a **kill switch**.
**Run against TESTNET only in this phase.**
**Done when:** the bot trades correctly on testnet for a full session with the risk layer
blocking anything out of bounds.
**Docs:** OMS state machine diagram, the full risk rule set, and an operational runbook
(how to start, stop, and emergency-halt).

### Phase 5 - Paper → tiny live + observability
**Build:** Prometheus metrics + a Grafana dashboard (PnL, positions, latency, error rates),
structured logs, and alerting. Run **paper/testnet for a sustained period**, then graduate
to **the smallest real size your exchange allows.**
**Done when:** you've run live small for long enough to trust the plumbing - not the profits.
**Docs:** deployment guide, monitoring guide, incident playbook.

### Phase 6 - Optimization & iteration
**Build:** profile and harden hot paths (zero-allocation parsing, lock-free queues, cache
-friendly layouts), add `criterion` benchmarks, expand to multiple exchanges/pairs, and do
**walk-forward analysis** rather than single-period backtests to fight overfitting.
**Done when:** measured latency improvements + a more robust validation process.
**Docs:** performance notes, benchmark results, the validation methodology.

---

## 5. Risk & money rules (memorize these)

- **`rust_decimal` everywhere** money is involved. Floats silently lose cents.
- **Testnet before mainnet. Paper before real. Tiny before scaled.** No exceptions.
- **A hard daily-loss kill switch** that flattens positions and halts trading.
- **Backtest honesty > backtest profit.** A pretty equity curve usually means a bug.
- **Reconcile state with the exchange on every (re)start** - never trust local-only state.
- **Secrets out of git.** API keys in env/secret store. Use API keys scoped to *trade
  only, no withdrawal*, with IP allowlisting where the exchange supports it.
- **Assume the strategy will eventually lose.** Size so a bad day is survivable and boring.

---

## 6. Suggested first week

1. `cargo new --workspace`, wire up CI, `tracing`, and the `core` types (Phase 0).
2. Stand up the mdBook and write ADR-0001 and the architecture overview.
3. Open a testnet account on one exchange; read its WebSocket docs.
4. Start Phase 1: stream trades, print them, then build the order book.

Ship Phase 0 + the first half of Phase 1, fully documented, before touching strategy logic.
The discipline you build here is the whole point.
