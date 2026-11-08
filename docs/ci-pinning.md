# CI action pinning

Third-party GitHub Actions run with the repository's token, so each one is a supply-chain
risk. The workflows pin them by full commit SHA (a version tag can be moved; a SHA cannot)
and keep the human-readable version in a comment next to it. Dependabot
(`.github/dependabot.yml`, ecosystem `github-actions`) proposes updates to the SHA and the
comment together.

## Status: SHAs checked against their tags

The SHAs live **only in the workflow files** (that is where Dependabot updates them, together
with the version comment); this page deliberately does not repeat them, because a copy drifts.
`scripts/verify-action-pins.sh` checks every pinned action: it must be a full 40-character SHA
with a `# vX.Y.Z` comment, and the SHA must be the commit that tag points to (`git ls-remote`,
no token needed). It was last run on 2026-11-08 and all pins matched. Run it after any bump:

```console
scripts/verify-action-pins.sh
```

That proves the tag-to-commit mapping. It does not prove the action's code is safe; for a new
action or a major bump, read the release diff (checklist below).

| Action | Used in |
|---|---|
| `actions/checkout` | ci, nightly, dco, fuzz, mutants, bench-compare, release |
| `Swatinem/rust-cache` | ci, nightly |
| `taiki-e/install-action` | ci (`deny`, `text`), nightly, fuzz, mutants |
| `actions/upload-artifact`, `actions/download-artifact` | mutants, bench-compare, release |
| `rust-lang/crates-io-auth-action`, `pypa/gh-action-pypi-publish` | release (trusted publishing) |
| `actions/setup-python`, `actions/setup-node` | bench-compare, release (v7.1.0, which run on node24) |

`bench-compare.yml` is not part of `ci-success`, installs third-party packages (PyPI, npm,
crates.io) with `--ignore-scripts` where possible, and has `contents: read` only.

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
| `cargo-deny` | `0.20.2` | yes, in `ci.yml` and `nightly.yml` (`cargo deny --locked check` passes locally with this version; 0.18.3 cannot read advisories that use CVSS 4.0) |
| `actionlint` | `1.7.7` | yes, in `ci.yml`: the release tarball, verified against the SHA-256 in the release's checksums file (taiki-e/install-action does not know actionlint). `actionlint` passes on all workflows locally |
| `gitleaks` | `8.30.1` | yes, in `ci.yml` (`gitleaks` job): `go install github.com/zricethezav/gitleaks/v8@v8.30.1`, verified by Go's checksum database. The module still declares the author's original import path. Scans the full history and fails on any finding. Checked locally: no findings in this repository, and a planted token-shaped secret is found (also after its file is deleted in a later commit) |
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
5. Add the row to the table above, then run `scripts/verify-action-pins.sh`.
