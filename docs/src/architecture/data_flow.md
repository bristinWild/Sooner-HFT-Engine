# Data Flow - Detailed

## MarketEvent lifecycle

```text
Exchange WebSocket frame (raw bytes)
         │
         ▼
  tokio-tungstenite → Message::Text(json_string)
         │
         ▼
  serde_json::from_str::<BybitWsMessage>()
  [types.rs - wire structs]
         │
         ▼
  parser::parse_trade() / parse_orderbook()
  - string → Decimal (price, qty)
  - ms timestamp → OffsetDateTime
  - "Buy"/"Sell" → Side enum
  [parser.rs - the boundary]
         │
         ▼
  MarketEvent { instrument, kind: Trade | OrderBookUpdate }
  [hft-core domain type]
         │
         ▼
  mpsc::Sender<MarketEvent>::send()
  [bounded channel, capacity 1024]
         │
         ▼
  Consumer: mpsc::Receiver<MarketEvent>::recv()
```

## Order book update lifecycle

```text
MarketEvent::OrderBookUpdate(update)
         │
         ▼
  LocalOrderBook::apply(&update)
  │
  ├── is_snapshot=true  → clear + rebuild from update.bids/asks
  │
  └── is_snapshot=false → validate sequence (last_seq + 1 == update.seq)
       │                    └── gap? → clear book, set initialised=false
       │
       └── apply_level() for each bid/ask
            ├── qty == 0 → BTreeMap::remove(price)
            └── qty  > 0 → BTreeMap::insert(price, qty)
         │
         ▼
  book.best_bid() / best_ask() / spread() / mid_price()
  [available to strategy immediately after apply()]
```

## Timestamp flow

Every event carries two timestamps for latency measurement:

```text
exchange_ts: set by the exchange when the event occurred
local_ts:    set by our parser when we processed the message

latency = local_ts - exchange_ts

Typical values:
  testnet over home connection: 20–90ms
  mainnet, cloud VM same region: 1–5ms
  mainnet, co-located:          <1ms
```

Track this metric in Phase 5 via Prometheus. Sudden latency spikes
indicate network issues and may warrant pausing trading.