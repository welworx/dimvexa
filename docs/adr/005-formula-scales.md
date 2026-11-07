# ADR-005: Formula scales (Baume, API, Brix) live in an optional pack

Status: Proposed - owner decision needed

## Context

These scales are defined by published formulas or tables, not by one factor, so they are not pure rationals.

## Decision

Recommended option (b): ship them in an optional `scales` pack where each scale carries its published definition and an explicit exactness label, decided per scale. The base package stays purely rational. Nothing in M0 implements this; the Shape type is closed to Linear and Affine for now.

## Consequences

Keeps the core simple. The owner must confirm the option before M2.
