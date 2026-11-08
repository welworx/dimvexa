#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Checks that every third-party action in .github/workflows/ is pinned to a full commit SHA and
# that the SHA is the commit its version comment names (`# v1.2.3`). Needs network access to
# github.com (git protocol only; no API token). Run it after Dependabot or a manual bump:
#   scripts/verify-action-pins.sh
# Exit code 1 on a placeholder, an unpinned reference or a SHA that does not match its tag.
# It proves the tag-to-commit mapping, not that the action's code is safe: read the release diff
# for that (docs/ci-pinning.md, checklist).
set -euo pipefail
cd "$(dirname "$0")/.."
bad=0
while read -r ref ver; do
  repo="${ref%@*}"; sha="${ref#*@}"
  if ! [[ "$sha" =~ ^[0-9a-f]{40}$ ]]; then echo "NOT PINNED  $ref"; bad=1; continue; fi
  if ! [[ "$ver" =~ ^v[0-9] ]]; then echo "NO VERSION  $ref (add a '# vX.Y.Z' comment)"; bad=1; continue; fi
  # An annotated tag lists the tag object and, peeled with ^{}, the commit; a lightweight tag lists the commit.
  tags="$(git ls-remote "https://github.com/$repo.git" "refs/tags/$ver" "refs/tags/$ver^{}")"
  if [ -z "$tags" ]; then echo "NO SUCH TAG $repo $ver"; bad=1
  elif grep -q "^$sha" <<<"$tags"; then echo "ok          $repo $ver $sha"
  else echo "MISMATCH    $repo $ver pinned $sha, tag is: $(awk '{print $1}' <<<"$tags" | tr '\n' ' ')"; bad=1; fi
done < <(grep -rhoE 'uses: [A-Za-z0-9_.-]+/[A-Za-z0-9_./-]+@[^ ]+ *# *v[^ ]+' .github/workflows/*.yml \
         | sed -E 's/uses: //; s/ *# */ /' | sort -u)
# A pin with no version comment is not matched by the pattern above; report any such line.
if grep -rnE 'uses: [A-Za-z0-9_.-]+/[A-Za-z0-9_./-]+@[^ ]+ *$' .github/workflows/*.yml; then bad=1; fi
exit "$bad"
