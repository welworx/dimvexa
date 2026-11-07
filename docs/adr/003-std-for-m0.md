# ADR-003: Core uses std in M0; no_std deferred

Status: Accepted for M0

## Context

`no_std` plus `alloc` helps WASM size and embedded use but restricts the parser, registry (HashMap) and error types.

## Decision

M0 uses std. The numeric layer (Dim, Ratio, Conversion apply paths) does not depend on std features beyond alloc and could move to `no_std` later. No unsafe code is used in library crates.

## Consequences

Faster iteration now. Revisit in M2 with WASM size measurements; the registry would then need a hasher without std.
