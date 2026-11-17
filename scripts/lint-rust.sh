#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Runs the Rust lint gates of CI (`cargo fmt --check` and `clippy -D warnings`)
# against the toolchain that rust-toolchain.toml pins. It refuses to run when
# rustc is not exactly that version, so local work cannot miss lints that CI
# catches with a newer clippy.
#
# Usage: scripts/lint-rust.sh [-p <crate>]
# Without arguments it lints the whole workspace. Set LINT_RUST_CHECK_ONLY=1 to
# stop after the version gate.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly repo_root
cd "$repo_root"
readonly toolchain_file="$repo_root/rust-toolchain.toml"

pinned_version="$(sed -n 's/^channel = "\(.*\)"$/\1/p' "$toolchain_file")"
if [[ -z "$pinned_version" ]]; then
  echo "cannot read the channel from $toolchain_file" >&2
  exit 1
fi

rust_version="$(rustc --version)"
if [[ "$rust_version" != *"$pinned_version"* ]]; then
  echo "$rust_version is not the pinned $pinned_version; run this script through 'rustup run $pinned_version' or put the pinned toolchain first on PATH" >&2
  exit 1
fi

if [[ -n "${LINT_RUST_CHECK_ONLY:-}" ]]; then
  echo "version gate passed: $rust_version matches $pinned_version"
  exit 0
fi

if (( "$#" == 0 )); then
  set -- --workspace
fi

cargo fmt --all -- --check
cargo clippy "$@" --all-targets -- -D warnings
