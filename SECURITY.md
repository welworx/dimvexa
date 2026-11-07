# Security policy

## Supported versions

Only the latest `0.x` release receives fixes.

## Reporting a vulnerability

Please do not open a public issue. Use GitHub's private vulnerability reporting
(*Security > Report a vulnerability* on the repository page).

<!-- TODO(owner): enable private vulnerability reporting in the repository settings and add
a fallback contact address here. -->

Include a description, a minimal reproducer (for the parser, the input string) and the
affected version. You can expect an acknowledgement within a few days.

## Scope

The library is pure computation with no I/O and no `unsafe` code. Relevant issues are, for
example, panics, unbounded memory or time on crafted input, and wrong conversion factors.
A wrong factor in the unit table is a correctness bug with security-like priority: please
report it with the primary source.
