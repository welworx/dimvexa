# Governance

## Model

Dimvexa starts with a single maintainer (the project owner) who has final say. The goals
are fixed by `docs/PLAN.md`: correctness first, exact factors, loud failures, permissive
licensing.

## Decisions

- Small changes: a pull request. While the owner is the only maintainer, required approvals
  stay at 0 and the owner merges their own pull requests once `ci-success` is green
  (`docs/repo-settings.md`).
- Anything that changes the public API, a unit definition policy or an ADR: a pull request
  that updates or adds an ADR in `docs/adr/`, open for comment for at least a week.
- Data changes need review of the cited source by a second person when one is available.

## Adding a maintainer

A contributor with a sustained record of high-quality, reviewed contributions (code or
data) may be invited by the owner. New maintainers are added to `CODEOWNERS` and
`MAINTAINERS.md`, accept the Code of Conduct, and are expected to uphold the Definition of
Done in `CONTRIBUTING.md`.

## Sign-off

The Developer Certificate of Origin sign-off is required from outside contributors. The
owner (`welworx`) is exempt: the DCO check is skipped for pull requests the owner authors, and
coding agents working for the owner do not add `Signed-off-by` lines on their behalf.

## Releases

Releases are cut with `release-plz` from Conventional Commits (planned for M1). The owner
approves release pull requests.
