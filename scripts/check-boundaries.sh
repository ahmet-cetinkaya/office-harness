#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Asserts the dependency rules of docs/ARCHITECTURE.md ("Dependency Rules" and
# "Testing Strategy and Boundary Enforcement") against `cargo metadata`.
# A crate's role comes from its manifest path (src/modules/<module>/<layer>/,
# src/shared/, src/kernel/, src/apps/), never from its name.
#
# Usage: scripts/check-boundaries.sh [--self-test]
set -euo pipefail

# Reads cargo metadata JSON on stdin and prints one line per violation, each
# starting with "rule <number>:".
readonly VIOLATIONS_FILTER='
def normalize: gsub("\\\\"; "/");

def classify($root):
  (.manifest_path | normalize | ltrimstr($root | normalize | rtrimstr("/"))) as $path
  | {name: .name, dependencies: [.dependencies[].name]}
    + (
      if $path | test("^/src/modules/[^/]+/[^/]+/Cargo.toml$") then
        ($path | capture("^/src/modules/(?<module>[^/]+)/(?<layer>[^/]+)/Cargo.toml$")) + {kind: "module"}
      elif $path | test("^/src/shared/") then {kind: "shared"}
      elif $path | test("^/src/kernel/") then {kind: "kernel"}
      elif $path | test("^/src/apps/desktop/") then {kind: "desktop"}
      elif $path | test("^/src/apps/") then {kind: "app"}
      else {kind: "other"}
      end
    );

{
  domain: ["application", "infrastructure", "presentation"],
  application: ["infrastructure", "presentation"],
  infrastructure: ["presentation"],
  presentation: ["infrastructure"]
} as $outer_layers
| (.workspace_root // "") as $root
| .workspace_members as $members
| [.packages[] | select(.id as $id | $members | index($id)) | classify($root)] as $crates
| ($crates | map({(.name): .}) | add) as $by_name
| $crates[] as $crate
| $crate.dependencies[] as $dependency
| ($by_name[$dependency] // {kind: "external"}) as $target
| (
    (select($crate.kind == "module" and ($crate.layer | IN("domain", "application"))
        and ($dependency | IN("axum", "rusqlite", "portable-pty", "vt100", "tokio")))
      | "rule 1: \($crate.name) (\($crate.layer)) must not depend on \($dependency)"),
    (select($crate.kind == "module" and $crate.layer == "presentation"
        and ($dependency | IN("rusqlite", "portable-pty")))
      | "rule 2: \($crate.name) (presentation) must not depend on \($dependency)"),
    (select($crate.kind == "module" and $target.kind == "module" and $crate.module != $target.module)
      | "rule 3: \($crate.name) must not depend on another module crate, \($dependency)"),
    (select(($crate.kind | IN("kernel", "shared"))
        and ($target.kind | IN("module", "app", "desktop")))
      | "rule 4: \($crate.kind) crate \($crate.name) must not depend on the \($target.kind) crate \($dependency)"),
    (select($crate.kind == "shared" and $target.kind == "kernel")
      | "rule 4: shared crate \($crate.name) must not depend on kernel"),
    (select($crate.kind == "desktop"
        and ($target.kind == "module" or ($dependency | IN("rusqlite", "portable-pty"))))
      | "rule 5: desktop crate \($crate.name) must not depend on \($dependency)"),
    (select($crate.kind == "module" and $target.kind == "module" and $crate.module == $target.module
        and ($target.layer | IN($outer_layers[$crate.layer][])))
      | "rule 6: \($crate.name) (\($crate.layer)) must not depend on its own outer layer \($dependency)"),
    (select($crate.kind == "kernel" and ($dependency | IN("tokio", "axum", "rusqlite")))
      | "rule 7: kernel must not depend on \($dependency)"),
    (select($crate.kind == "module" and $crate.layer == "infrastructure" and $dependency == "axum")
      | "rule 8: \($crate.name) (infrastructure) must not depend on axum"),
    (select($crate.kind == "module" and ($target.kind | IN("app", "desktop")))
      | "rule 9: module crate \($crate.name) must not depend on the app crate \($dependency)")
  )
'

report_violations() {
  jq -r "$VIOLATIONS_FILTER"
}

# A minimal workspace that follows every rule.
clean_metadata() {
  cat <<'JSON'
{
  "workspace_root": "/w",
  "workspace_members": ["kernel", "wire", "agent-domain", "agent-application", "agent-infrastructure", "agent-presentation", "terminal-domain", "notification-domain", "notification-application", "notification-infrastructure", "notification-presentation", "office-harnessd", "desktop"],
  "packages": [
    {"id": "kernel", "name": "kernel", "manifest_path": "/w/src/kernel/Cargo.toml", "dependencies": [{"name": "serde"}]},
    {"id": "wire", "name": "wire", "manifest_path": "/w/src/shared/wire/Cargo.toml", "dependencies": [{"name": "serde"}]},
    {"id": "agent-domain", "name": "agent-domain", "manifest_path": "/w/src/modules/agent/domain/Cargo.toml", "dependencies": [{"name": "kernel"}]},
    {"id": "agent-application", "name": "agent-application", "manifest_path": "/w/src/modules/agent/application/Cargo.toml", "dependencies": [{"name": "agent-domain"}]},
    {"id": "agent-infrastructure", "name": "agent-infrastructure", "manifest_path": "/w/src/modules/agent/infrastructure/Cargo.toml", "dependencies": [{"name": "agent-application"}, {"name": "agent-domain"}, {"name": "rusqlite"}, {"name": "tokio"}]},
    {"id": "agent-presentation", "name": "agent-presentation", "manifest_path": "/w/src/modules/agent/presentation/Cargo.toml", "dependencies": [{"name": "agent-application"}, {"name": "wire"}]},
    {"id": "terminal-domain", "name": "terminal-domain", "manifest_path": "/w/src/modules/terminal/domain/Cargo.toml", "dependencies": [{"name": "kernel"}]},
    {"id": "notification-domain", "name": "notification-domain", "manifest_path": "/w/src/modules/notification/domain/Cargo.toml", "dependencies": [{"name": "kernel"}]},
    {"id": "notification-application", "name": "notification-application", "manifest_path": "/w/src/modules/notification/application/Cargo.toml", "dependencies": [{"name": "notification-domain"}]},
    {"id": "notification-infrastructure", "name": "notification-infrastructure", "manifest_path": "/w/src/modules/notification/infrastructure/Cargo.toml", "dependencies": [{"name": "notification-application"}, {"name": "notification-domain"}, {"name": "rusqlite"}]},
    {"id": "notification-presentation", "name": "notification-presentation", "manifest_path": "/w/src/modules/notification/presentation/Cargo.toml", "dependencies": [{"name": "notification-application"}, {"name": "notification-domain"}, {"name": "wire"}]},
    {"id": "office-harnessd", "name": "office-harnessd", "manifest_path": "/w/src/apps/office-harnessd/Cargo.toml", "dependencies": [{"name": "agent-presentation"}, {"name": "agent-infrastructure"}, {"name": "notification-presentation"}, {"name": "notification-infrastructure"}, {"name": "axum"}, {"name": "tokio"}]},
    {"id": "desktop", "name": "desktop", "manifest_path": "/w/src/apps/desktop/src-tauri/Cargo.toml", "dependencies": [{"name": "wire"}, {"name": "tauri"}]}
  ]
}
JSON
}

# The clean workspace as Windows reports it: drive letters and backslashes.
windows_style_metadata() {
  clean_metadata | jq '
    .workspace_root = "C:/w"
    | .packages |= map(.manifest_path |= ("C:" + gsub("/"; "\\")))'
}

# Prints the clean workspace with one forbidden dependency added to a crate.
planted_metadata() {
  local crate="$1" dependency="$2"
  "${3:-clean_metadata}" | jq --arg crate "$crate" --arg dependency "$dependency" \
    '(.packages[] | select(.name == $crate) | .dependencies) += [{"name": $dependency}]'
}

# Prints the distinct rule numbers that fired, space separated, on stdin metadata.
fired_rules() {
  report_violations | sed -E 's/^rule ([0-9]+):.*/\1/' | sort -un | tr '\n' ' ' | sed 's/ $//'
}

# Each planted case is "<crate> <forbidden dependency> <the one rule that must fire>".
readonly PLANTED_CASES=(
  "agent-domain rusqlite 1"
  "agent-domain tokio 1"
  "agent-application axum 1"
  "agent-application portable-pty 1"
  "agent-presentation portable-pty 2"
  "agent-presentation rusqlite 2"
  "agent-application terminal-domain 3"
  "kernel agent-domain 4"
  "wire kernel 4"
  "wire office-harnessd 4"
  "desktop agent-domain 5"
  "desktop rusqlite 5"
  "desktop portable-pty 5"
  "agent-domain agent-application 6"
  "agent-domain agent-infrastructure 6"
  "agent-application agent-infrastructure 6"
  "agent-application agent-presentation 6"
  "agent-presentation agent-infrastructure 6"
  "agent-infrastructure agent-presentation 6"
  "kernel tokio 7"
  "kernel axum 7"
  "kernel rusqlite 7"
  "agent-infrastructure axum 8"
  "agent-domain office-harnessd 9"
  "notification-domain rusqlite 1"
  "notification-application tokio 1"
  "notification-presentation rusqlite 2"
  "notification-application agent-domain 3"
  "notification-domain notification-infrastructure 6"
  "notification-infrastructure notification-presentation 6"
  "notification-domain office-harnessd 9"
)

check_planted_case() {
  local metadata_source="$1" label="$2" crate="$3" dependency="$4" expected="$5" fired
  fired="$(planted_metadata "$crate" "$dependency" "$metadata_source" | fired_rules)"
  if [[ "$fired" != "$expected" ]]; then
    echo "self-test FAILED: ${label:+$label }$crate -> $dependency fired rules [${fired:-none}], expected [$expected]" >&2
    return 1
  fi
  echo "self-test: ${label:+$label }$crate -> $dependency fired rule $fired"
}

self_test() {
  local failures=0 case_fields
  if [[ -n "$(clean_metadata | report_violations)" ]]; then
    echo "self-test FAILED: the clean workspace reported violations" >&2
    failures=1
  fi
  if [[ -n "$(windows_style_metadata | report_violations)" ]]; then
    echo "self-test FAILED: the clean workspace with Windows paths reported violations" >&2
    failures=1
  fi

  local planted
  for planted in "${PLANTED_CASES[@]}"; do
    read -r -a case_fields <<<"$planted"
    check_planted_case clean_metadata "" "${case_fields[@]}" || failures=1
  done
  check_planted_case windows_style_metadata "windows paths:" agent-domain rusqlite 1 || failures=1
  check_planted_case windows_style_metadata "windows paths:" agent-application agent-infrastructure 6 || failures=1

  if ((failures == 0)); then
    echo "self-test passed: the clean workspace is accepted and every planted dependency fires exactly its rule"
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
