# ADR-0004: BTreeMap for local order book storage

**Date:** Phase 1  
**Status:** Accepted

---

## Context

We need a data structure to store order book price levels where we
frequently need: insert, delete, lookup by price, and finding the
best bid (highest price) and best ask (lowest price).

## Decision

Two `BTreeMap<Price, Qty>` - one for bids, one for asks.

## Consequences

`BTreeMap` keeps keys sorted at all times. This means:

- `best_bid()` = `keys().next_back()` - O(log n)
- `best_ask()` = `keys().next()` - O(log n)
- Insert/delete = O(log n)
- Iteration in price order is free - useful for depth calculations

For 50 levels (our current subscription depth) this is faster and
simpler than a sorted `Vec` which requires O(n) shifts on insert/delete.

## Alternatives considered

**`Vec<BookLevel>` with binary search:** O(log n) lookup but O(n)
insert/delete due to shifting. Gets worse as depth increases.

**`HashMap<Price, Qty>`:** O(1) insert/delete/lookup but no ordering -
finding best bid/ask requires a full scan every time. Unsuitable.

**Custom skip list or B-tree:** Overkill for 50 levels. Revisit if
we subscribe to deeper books (500+ levels) and profiling shows BTreeMap
as a bottleneck.