# GitHub settings to apply by hand

Settings cannot be committed, so the owner applies them once in the repository settings on
GitHub. The repository has a single maintainer, so several defaults that suit teams are
deliberately **off** here.

## Branch protection for `main`

*Settings > Branches > Add branch ruleset (or classic rule) for `main`:*

- Require a pull request before merging, with **required approvals set to 0** (you are the only
  maintainer; a required approval would block every merge).
- **Do not** enable "Require review from Code Owners". `CODEOWNERS` lists you for every path, so
  the rule would demand a review from yourself.
- Require status checks to pass, and select **only `ci-success`**. It aggregates every other job,
  so adding or renaming a job later needs no settings change. Also require the branch to be up
  to date if you want it.
- Require **linear history** (squash or rebase merges; the pull request title becomes the
  commit message, which is why `pr-title` checks it).
- **Block force pushes** and **block deletions**.
- Do not enable "Do not allow bypassing the above settings" if you want to be able to fix a
  broken workflow directly; if you do enable it, you need a pull request for every fix.

## Security

*Settings > Code security:*

- **Private vulnerability reporting: enable.** `SECURITY.md` and the issue chooser link to
  `https://github.com/welworx/dimvexa/security/advisories/new`, which only works when this is on.
- **Dependabot alerts** and **Dependabot security updates**: enable. (Version updates are
  configured in `.github/dependabot.yml`.)
- **Secret scanning** and **push protection**: enable.

## Actions

*Settings > Actions > General:*

- Workflow permissions: **Read repository contents** (the default), do not allow Actions to
  create or approve pull requests. The workflows ask for anything more themselves (only the
  nightly alert job asks for `issues: write`).
- Fork pull request workflows: require approval for first-time contributors.

## Pages

*Settings > Pages:* Source **GitHub Actions**. (No docs workflow exists yet; the book and its
deploy workflow are planned for M1. Select the source when that workflow lands.)

## About box

- Description: `Exact, fast unit conversion for Rust: loud failures, zero guessing`
- Topics: `unit-conversion`, `units`, `rust`, `measurement`, `si-units`, `physical-units`,
  `engineering`
- Website: leave empty until docs are published.

## Social preview

Settings → General → Social preview → Edit → Upload an image: `docs/brand/github-social-preview.png`
(1280 x 640). GitHub has no way to set it from the repository, so it is a manual step. For the
npm organisation and the PyPI project avatar use `docs/brand/icon-256.png` or `icon-512.png`; the
brand rules are in `docs/brand/DESIGN_GUIDELINES.md`. The registries have no icon field, so the logo reaches the crates.io, PyPI and npm pages only through each README (an image served from this repository's `main` branch), and docs.rs through `#![doc(html_logo_url, html_favicon_url)]` in each crate; the avatars of the npm organisation and the PyPI account are set by hand.

## Labels

Create the labels the issue forms apply: `bug`, `enhancement`, `unit-request`, `needs-source`.
