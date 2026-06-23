# System Architecture Overview

```text
┌─────────────────────────────────────────────────────────┐
│                    hft-crypto engine                     │
│                                                         │
│  ┌──────────────┐    MarketEvent     ┌───────────────┐  │
│  │   exchanges  │ ──────────────────►│   strategy    │  │
│  │              │                    │  (Phase 3)    │  │
│  │ Bybit WS     │ ──────────────────►│               │  │
│  │ connector    │                    └───────┬───────┘  │
│  │              │ ──────────────────►        │           │
│  └──────────────┘    (channel)      OrderIntent         │
│                                              │           │
│  ┌──────────────┐                   ┌───────▼───────┐  │
│  │ persistence  │◄──────────────────│   execution   │  │
│  │  (Phase 2)   │   MarketEvent     │  OMS + Risk   │  │
│  │              │                   │  (Phase 4)    │  │
│  └──────────────┘                   └───────┬───────┘  │
│                                             │           │
└─────────────────────────────────────────────┼───────────┘
                                              │ REST/WS
                                         ┌───▼────┐
                                         │ Bybit  │
                                         │Exchange│
                                         └────────┘
```

## Crate dependency graph

```text
apps/trader ──────────────────────────────────────────►┐
apps/recorder ────────────────────────────────────────►┤
apps/backtester ──────────────────────────────────────►┤
                                                       │
crates/exchanges ─────────────────────────────────────►┤
crates/execution (Phase 4) ───────────────────────────►┤──► crates/hft-core
crates/strategy  (Phase 3) ───────────────────────────►┤
crates/backtest  (Phase 2) ───────────────────────────►┤
crates/persistence (Phase 2) ─────────────────────────►┘
```

**The rule:** arrows point toward `hft-core`. No arrows point out of it.
`hft-core` has no dependencies inside this repo.

## Data flow - live trading (Phase 5+)

```text
Bybit WebSocket
      │
      ▼
  Connector (crates/exchanges)
  - parses raw JSON
  - validates sequence numbers
  - emits MarketEvent on channel
      │
      ├──► LocalOrderBook.apply()   - maintains live book state
      │
      ├──► Recorder                 - writes events to disk
      │
      └──► Strategy.on_event()      - generates OrderIntents
                │
                ▼
          RiskManager               - validates intent against limits
                │
                ▼
             OMS                    - manages order lifecycle
                │
                ▼
         Bybit REST/WS              - places/cancels orders
```

## Data flow - backtesting (Phase 2+)

```text
Recorded events (disk)
      │
      ▼
  EventReplayer                     - same channel interface as connector
      │
      └──► (identical path as live trading from here)
```

The backtester uses the **same** `Strategy`, `RiskManager`, and `OMS`
code as live trading. The only difference is the event source.
If the backtest and live paths diverged, the backtest would be useless.

## Event-driven model

Components never call each other directly. They communicate through
typed events on bounded channels. This means:

- Adding a new consumer (monitoring, secondary strategy) = subscribe
  to the channel. No changes to the producer.
- Testing a strategy = send synthetic `MarketEvent`s. No network needed.
- Backtest reuse = replace the WebSocket source with a file reader.