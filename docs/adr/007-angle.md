# ADR-007: Plane angle is dimensionless

Status: Accepted (amended by [ADR-020](020-angle-and-cycle-kinds.md): angle stays dimensionless, but `rad/s` and `Hz` no longer convert)

## Context

UCUM treats angle as a base dimension; SI treats radian as dimensionless. Hz and rad/s should stay compatible.

## Decision

Angle is dimensionless by default, diverging from UCUM on purpose. The divergence is documented. A UCUM conformance mode is a later option.

## Consequences

Hz and rad/s interoperate. UCUM conformance tests that depend on angle need an explicit exception.
