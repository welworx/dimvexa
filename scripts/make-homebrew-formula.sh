#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Prints the Homebrew formula for a release to stdout. Usage:
#   scripts/make-homebrew-formula.sh <version> <dir with dimvexa-<version>-<target>.tar.gz(.sha256)>
# The release workflow attaches the result to the GitHub release as dimvexa.rb; copy it to
# Formula/dimvexa.rb in the tap repo (welworx/homebrew-tap). See docs/releasing.md.
set -euo pipefail
v="${1:?version}"; dir="${2:?directory with the .sha256 files}"
sha() {
  local h; h=$(awk '{print $1}' "$dir/dimvexa-$v-$1.tar.gz.sha256")
  [[ "$h" =~ ^[0-9a-f]{64}$ ]] || { echo "bad or missing sha256 for $1" >&2; exit 1; }
  echo "$h"
}
a=$(sha aarch64-apple-darwin); b=$(sha x86_64-apple-darwin)
c=$(sha aarch64-unknown-linux-musl); d=$(sha x86_64-unknown-linux-musl)
base="https://github.com/welworx/dimvexa/releases/download/v$v/dimvexa-$v"
cat <<RB
class Dimvexa < Formula
  desc "Unit conversion with exact factors and loud failures"
  homepage "https://github.com/welworx/dimvexa"
  version "$v"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    on_arm do
      url "$base-aarch64-apple-darwin.tar.gz"
      sha256 "$a"
    end
    on_intel do
      url "$base-x86_64-apple-darwin.tar.gz"
      sha256 "$b"
    end
  end

  on_linux do
    on_arm do
      url "$base-aarch64-unknown-linux-musl.tar.gz"
      sha256 "$c"
    end
    on_intel do
      url "$base-x86_64-unknown-linux-musl.tar.gz"
      sha256 "$d"
    end
  end

  def install
    bin.install "dimvexa"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/dimvexa --version")
  end
end
RB
