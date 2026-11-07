# CI action pinning

Third-party GitHub Actions run with the repository's token, so each one is a supply-chain
risk. The workflows pin them by full commit SHA (a version tag can be moved; a SHA cannot)
and keep the human-readable version in a comment next to it. Dependabot
(`.github/dependabot.yml`, ecosystem `github-actions`) proposes updates to the SHA and the
comment together.

## Status: copied, not verified

The authoring session had no access to GitHub, so **no SHA below was looked up or checked
by us.** They were copied from the workflow of a large, widely used Rust repository
(`tangency-labs/solidcraft`) and are labelled in the workflow files with the comment
"copied from a widely used repo, verify against the action's release page before enabling".

**Before enabling the workflows**, for each row: open the action's releases page, find the
release named in the "Version comment" column, and check that the commit it points to is the
SHA in the table. If a SHA does not match, replace it with the one on the release page, or the
step will fail with "unable to find version" (the safe failure). Do the same for the
`cargo-deny` version.

| Action | SHA (full) | Version comment | Used in |
|---|---|---|---|
| `actions/checkout` | `3d3c42e5aac5ba805825da76410c181273ba90b1` | v7.0.1 | ci, nightly, dco, fuzz, mutants |
| `Swatinem/rust-cache` | `6323deb102c322ba6fcbdcafc7e3dddab59af2b6` | v2.9.2 | ci, nightly |
| `taiki-e/install-action` | `1168f6810a3cb30a3514bf5756a3b7f0422c6df8` | v2.87.28 | ci (`deny`, `text`, `workflows`), nightly, fuzz, mutants |
| `actions/upload-artifact` | `cf430e030ddbb5b0abf93d22962f4752f3646cd9` | v7.0.2 | mutants |

## What is not an action

* **Rust toolchain.** Installed with `rustup` (preinstalled on GitHub-hosted runners) in a
  `run:` step, instead of `dtolnay/rust-toolchain`. That action is normally referenced by a
  moving ref (`@stable`), and no commit SHA for it was available to copy, so no unverified or
  floating third-party action is used for it. The MSRV job selects `1.85` through
  `RUSTUP_TOOLCHAIN`.
* **Pull request title check.** A shell step with a regular expression (`pr-title.yml`),
  instead of `amannn/action-semantic-pull-request`.
* **DCO check.** A shell step over `git log` (`dco.yml`).
* **Aggregate job.** A shell step that reads the `needs` context with `jq`.

## Tool versions installed by `taiki-e/install-action`

| Tool | Version | Pinned? |
|---|---|---|
| `cargo-deny` | `0.18.3` | yes, in `ci.yml` and `nightly.yml` (unverified: confirm the release exists and that `deny.toml` is accepted by it) |
| `actionlint` | `1.7.7` | yes, in `ci.yml` (unverified: confirm the release exists) |
| `cargo-fuzz`, `cargo-mutants`, `cargo-llvm-cov` | latest | no (weekly and nightly workflows only) |
| `typos-cli`, `taplo-cli` | latest | no. Choose versions and pin them (`typos-cli@X.Y.Z`) once the first run is green |

## Checklist for adding or changing an action

1. Prefer a `run:` step with a tool that is already on the runner.
2. If an action is needed, pin the full 40-character SHA of a release and put the version in a
   comment on the same line.
3. Read the action's source or the release diff for what it does with `GITHUB_TOKEN` and with
   secrets. Give the job only the `permissions:` it needs (the default here is
   `contents: read`).
4. Never use `pull_request_target`, and never interpolate `${{ github.event.* }}` text
   (titles, branch names, bodies) into a script; pass it through `env:`.
5. Add the row to the table above.
