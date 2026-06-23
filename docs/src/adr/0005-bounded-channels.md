# ADR-0005: Bounded async channels for event passing

**Date:** Phase 1  
**Status:** Accepted

---

## Context

Components communicate by passing `MarketEvent`s through tokio channels.
We must choose between bounded and unbounded channels.

## Decision

All channels in the system are **bounded**. The connector uses a buffer
of 1024 events by default, configurable at spawn time.

## Consequences

**Backpressure:** if the consumer (strategy, recorder) processes events
slower than the connector produces them, the channel fills up and the
connector's `send` call blocks. The system slows down rather than
accumulating unbounded memory.

**Why this protects you:** an unbounded channel during a fast market
(e.g. a flash crash with thousands of updates per second) would silently
grow until the process runs out of memory and crashes - at exactly the
moment you need it most.

**The right buffer size:** too small = artificial latency as the producer
waits; too large = delayed backpressure signal. 1024 is a starting point.
Phase 5 adds metrics to measure fill level and tune this number.

## Alternatives considered

**`tokio::sync::mpsc` unbounded:** rejected - no backpressure, unbounded
memory growth under load.

**`flume` bounded:** viable alternative with slightly better performance
in some benchmarks. `tokio::mpsc` is sufficient and avoids an extra
dependency for now.