#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Asserts the dependency rules of docs/ARCHITECTURE.md ("Testing Strategy and
# Boundary Enforcement") against `cargo metadata`.
#
# Usage: scripts/check-boundaries.sh [--self-test]
set -euo pipefail

# Reads cargo metadata JSON on stdin and prints one line per violation.
readonly VIOLATIONS_FILTER='
def parts: capture("^(?<module>[a-z]+)-(?<layer>domain|application|infrastructure|presentation)$") // null;
def is_module_crate: parts != null;

.workspace_members as $members
| .packages[]
| select(.id as $id | $members | index($id))
| .name as $crate
| (.manifest_path | gsub("\\\\"; "/")) as $path
| ($crate | parts) as $own
| [.dependencies[].name] as $deps
| (
    ($deps[] | select(IN("axum", "rusqlite", "portable-pty", "vt100", "tokio"))
      | select($own != null and ($own.layer == "domain" or $own.layer == "application"))
      | "rule 1: \($crate) must not depend on \(.)"),
    ($deps[] | select(IN("rusqlite", "portable-pty"))
      | select($own != null and $own.layer == "presentation")
      | "rule 2: \($crate) must not depend on \(.)"),
    ($deps[] | select(is_module_crate) | select(($own != null) and (parts.module != $own.module))
      | "rule 3: \($crate) must not depend on another module crate, \(.)"),
    ($deps[] | select(is_module_crate or . == "kernel")
      | select($path | contains("/src/shared/"))
      | "rule 4: shared crate \($crate) must not depend on \(.)"),
    ($deps[] | select(is_module_crate)
      | select($crate == "kernel")
      | "rule 4: kernel must not depend on \(.)"),
    ($deps[] | select(is_module_crate)
      | select($path | contains("/src/apps/desktop/"))
      | "rule 5: desktop crate \($crate) must not depend on \(.)")
  )
'

report_violations() {
  jq -r "$VIOLATIONS_FILTER"
}

# A minimal workspace that follows every rule.
clean_metadata() {
  cat <<'JSON'
{
  "workspace_members": ["kernel", "wire", "agent-domain", "agent-application", "agent-presentation", "terminal-domain", "desktop"],
  "packages": [
    {"id": "kernel", "name": "kernel", "manifest_path": "/w/src/kernel/Cargo.toml", "dependencies": [{"name": "serde"}]},
    {"id": "wire", "name": "wire", "manifest_path": "/w/src/shared/wire/Cargo.toml", "dependencies": [{"name": "serde"}]},
    {"id": "agent-domain", "name": "agent-domain", "manifest_path": "/w/src/modules/agent/domain/Cargo.toml", "dependencies": [{"name": "kernel"}]},
    {"id": "agent-application", "name": "agent-application", "manifest_path": "/w/src/modules/agent/application/Cargo.toml", "dependencies": [{"name": "agent-domain"}]},
    {"id": "agent-presentation", "name": "agent-presentation", "manifest_path": "/w/src/modules/agent/presentation/Cargo.toml", "dependencies": [{"name": "agent-application"}, {"name": "wire"}]},
    {"id": "terminal-domain", "name": "terminal-domain", "manifest_path": "/w/src/modules/terminal/domain/Cargo.toml", "dependencies": [{"name": "kernel"}]},
    {"id": "desktop", "name": "desktop", "manifest_path": "/w/src/apps/desktop/src-tauri/Cargo.toml", "dependencies": [{"name": "wire"}]}
  ]
}
JSON
}

# Prints the clean workspace with one forbidden dependency added to a crate.
planted_metadata() {
  local crate="$1" dependency="$2"
  clean_metadata | jq --arg crate "$crate" --arg dependency "$dependency" \
    '(.packages[] | select(.name == $crate) | .dependencies) += [{"name": $dependency}]'
}

self_test() {
  local failures=0
  if [[ -n "$(clean_metadata | report_violations)" ]]; then
    echo "self-test FAILED: the clean workspace reported violations" >&2
    failures=1
  fi

  local planted
  for planted in agent-domain:rusqlite agent-application:tokio agent-presentation:portable-pty \
    agent-application:terminal-domain kernel:agent-domain wire:kernel desktop:agent-domain; do
    if [[ -z "$(planted_metadata "${planted%%:*}" "${planted#*:}" | report_violations)" ]]; then
      echo "self-test FAILED: planted dependency ${planted%%:*} -> ${planted#*:} was not detected" >&2
      failures=1
    fi
  done

  if ((failures == 0)); then
    echo "self-test passed: the clean workspace is accepted and every planted dependency is rejected"
  fi
  return "$failures"
}

check_workspace() {
  local violations
  violations="$(cargo metadata --format-version 1 --no-deps | report_violations)"
  if [[ -n "$violations" ]]; then
    echo "$violations" >&2
    return 1
  fi
  echo "boundaries ok"
}

case "${1:-}" in
  --self-test) self_test ;;
  "") check_workspace ;;
  *) echo "usage: $0 [--self-test]" >&2; exit 2 ;;
esac
