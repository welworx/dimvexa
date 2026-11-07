# ADR-001: No equations of state or empirical correlations

Status: Accepted

## Context

A process-engineering consumer will want conversions such as standard volume to actual volume, mass to mole, or gauge to absolute pressure. Several need an equation of state, a density or a composition, and a silent guess would give wrong numbers that look right.

## Decision

The library performs no conversion that needs an equation of state, a density, a composition or an empirical formula. Standard volumes (Nm3, Sm3, scf) are separate units, each with an exact stated reference condition. Converting between reference conditions, or to an actual volume, is refused with a clear error unless the caller passes an explicit factor. Mass, mole and volume fractions never convert into each other without caller-supplied data.

## Consequences

Failures are loud and the core stays purely rational. Users must bring their own thermodynamics. Callers who want the convenience do it in a separate crate that depends on this one.
