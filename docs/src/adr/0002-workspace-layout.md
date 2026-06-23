# ADR-0002: Cargo workspace with `crates/` and `apps/` separation

**Date:** Phase 0  
**Status:** Accepted

---

## Context

We need to organise multiple Rust crates that share types and depend
on each other.

## Decision

A Cargo workspace with two top-level groupings:

- `crates/` - reusable libraries (no `main()`).
- `apps/` - runnable binaries that depend on the libraries.

`hft-core` is the foundation. The dependency graph must be a DAG -
no circular dependencies allowed.

## Consequences

- Single `cargo test` runs all crates in dependency order.
- Shared `[workspace.dependencies]` prevents version skew.
- The compiler enforces boundaries - if you want to use a type from
  `execution` in `marketdata`, it's a compile error (that would be
  a cycle). Shared types must live in `hft-core`.
- `hft-core` cannot import `tokio` or do IO - enforced by the fact
  that those crates aren't in its `Cargo.toml`.

## Alternatives considered

- **Single crate with modules**: no compiler-enforced boundaries between
  modules. Easy to create implicit coupling. Rejected.
- **Separate repositories**: too much overhead for one project. Rejected.