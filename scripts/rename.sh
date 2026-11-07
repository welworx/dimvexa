#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Rename the project in one step: scripts/rename.sh newname
# Replaces the lowercase name `dimvexa`, the capitalised `Dimvexa` and the
# upper-case `DIMVEXA` in all tracked text files and renames crate directories.
set -euo pipefail
new="${1:?usage: scripts/rename.sh <new-lowercase-name>}"
cap="$(tr '[:lower:]' '[:upper:]' <<<"${new:0:1}")${new:1}"
up="$(tr '[:lower:]' '[:upper:]' <<<"$new")"
cd "$(git rev-parse --show-toplevel)"
git ls-files -z | grep -zv -e '^LICENSE-' -e '^scripts/rename.sh$' | xargs -0 sed -i \
  -e "s/dimvexa/${new}/g" -e "s/Dimvexa/${cap}/g" -e "s/DIMVEXA/${up}/g"
for d in crates/dimvexa*; do
  [ -d "$d" ] && git mv "$d" "${d/dimvexa/$new}"
done
echo "Renamed to ${new}. Review with git diff, then run cargo build and cargo test."
