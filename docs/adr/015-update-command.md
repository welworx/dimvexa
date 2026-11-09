# ADR-015: `dimvexa update` replaces the binary from the GitHub release, using curl and tar

Status: Proposed (implemented on the branch; the status moves to Accepted only when the owner says
so, and the OPEN items below record what was defaulted)

Amends ADR-003 only in that the CLI runs two system tools; the dependency policy is unchanged.

## Context

`flatex-pdf-cli upgrade [-check] [-y]` (same owner, Go) asks the GitHub API for the latest release,
picks the asset for the platform, verifies it against `SHA256SUMS.txt`, and swaps the running
binary by renaming it aside (which also works on Windows). Users of the prebuilt `dimvexa` binary
(ADR in `docs/releasing.md`: tarballs per target, a `.sha256` per file, a loose `dimvexa.exe`) have
no way to update except downloading by hand.

The workspace has no third-party dependencies and builds offline (AGENTS.md, ADR-003), and the
library rules forbid `unwrap`, indexing and unchecked arithmetic. The standard library has no HTTP
client, no TLS, no gzip and no SHA-256.

## Decision

1. **Command:** `dimvexa update [--check] [-y|--yes]`. It is not a conversion and has no JSON
   output. `--check` exits 0 (up to date), 3 (newer release available) or 1 (error); a usage error
   is 2. The code 3 is new and documented in `--help`. flatex uses 1 for "available" and 2 for
   errors, but here 1 and 2 already mean "refused" and "usage".
2. **Which release:** the redirect target of `https://github.com/welworx/dimvexa/releases/latest`
   (`.../releases/tag/vX.Y.Z`), so no API call, no rate limit and no JSON parser. The tag must be a
   plain version; it is put into URLs. Only a strictly newer version installs (no downgrade).
3. **Download and check:** the asset the release workflow builds for this target
   (`dimvexa-X.Y.Z-<triple>.tar.gz`, or `dimvexa.exe` on Windows) and its `.sha256` file. The hash
   is computed in a small std-only SHA-256 (tested against the FIPS 180-4 vectors) and must match
   the file named in the `.sha256` file. A mismatch installs nothing.
4. **Tools:** the system `curl` (HTTPS only, also after redirects, TLS 1.2 or newer) and, on
   Unix, `tar`. Both ship with macOS, Windows 10+ and mainstream Linux. A missing tool is an error
   that says so. No dependency is added.
5. **Replace:** copy next to the running binary, rename the running binary to `.old`, rename the
   copy into place, remove `.old`; on failure put the old binary back. On Windows the running
   `.exe` cannot be deleted, so `dimvexa.exe.old` stays until the next update removes it.
6. **Package-managed installs are refused:** a path under Homebrew's Cellar, `/opt/homebrew`, `/home/linuxbrew`, `.cargo/bin` or
   `$CARGO_HOME/bin` gets
   the right command (`brew upgrade dimvexa`, `cargo install dimvexa-cli --force`) instead of an
   in-place overwrite that the package manager would undo or trip over. The detection is by path,
   so other installers (Scoop, winget, `cargo install --root`, distro packages) are not recognised;
   the user is expected to use their tool.
   No answer on stdin (end of input without `-y`) is an error, not an abort, so a cron job or CI
   step cannot report success without updating. `--check` still works.
7. **Nothing implicit:** no background check, no update notice on other commands, no environment
   variable that changes the source. The command only runs when asked.

## Consequences

* Prebuilt-binary users update with one command. The integrity guarantee is the one flatex has:
  the checksum comes from the same GitHub release over TLS, so it detects a damaged or truncated
  download, not a compromised release. Signed releases would be a separate decision.
* The CLI now runs external programs and talks to the network, which no other command does.
  Everything else (conversion, units, configs) stays offline.
* The flow is tested offline against a fake release (a real `.tar.gz` on Unix); the real `curl`
  path is checked by hand at release time, because CI is not allowed to depend on a release.
* Linux binaries are musl builds, so they run on glibc systems too. Other platforms (for example
  FreeBSD) get an error that points to `cargo install`.

## OPEN

* Should `update` also be allowed for a cargo-installed binary (`-f`)? Default: no.
* Should releases carry signatures (minisign or sigstore) so that `update` can verify authenticity
  rather than integrity? Default: not now.
