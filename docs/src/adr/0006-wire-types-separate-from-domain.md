# ADR-0006: Separate wire types from domain types

**Date:** Phase 1  
**Status:** Accepted

---

## Context

Exchange APIs send data in their own formats - Bybit uses single-letter
field names (`T`, `s`, `S`, `v`, `p`), string prices, millisecond
timestamps, and exchange-specific envelopes. We need to decide where
exchange-specific knowledge lives.

## Decision

Two strictly separated layers per connector:

- `types.rs` - raw structs that mirror the exchange wire format exactly.
  Named to match the API docs. Only used inside the connector.
- `parser.rs` - converts wire types to `MarketEvent` domain types.
  The only place string-to-Decimal parsing happens.

Domain types (`MarketEvent`, `Trade`, `OrderBookUpdate`) know nothing
about any specific exchange.

## Consequences

**Isolation:** when Bybit changes a field name or adds a new message
type, only `types.rs` and `parser.rs` need updating. Strategy code,
the order book, and the recorder are unaffected.

**Testability:** parsers can be unit-tested with hardcoded JSON strings
without a network connection.

**The boundary rule:** string prices become `Decimal` exactly once, in
`parser.rs`, at the point of entry into the system. Nowhere else in
the codebase parses price strings.

## Alternatives considered

**Single struct with serde attributes:** would mix wire concerns into
domain types, making it impossible to support multiple exchanges with
the same domain model without conditional compilation or enums
everywhere. Rejected.