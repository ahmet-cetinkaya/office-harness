# QA report: agent and team templates (0009)

Status: in progress. T6 wrote the crate facts below. T8 adds the rows for SC-001 to SC-005 and the edge cases.

## T6: crate facts for `tauri-plugin-dialog` (plan D6, conditions d and e)

Date: 2026-11-22. Toolchain 1.100.0. Each fact comes from `cargo metadata --format-version 1 --locked` or `cargo tree --locked`, not from the plan.

| Fact | Plan said | Found |
|---|---|---|
| Version in `Cargo.lock` | `2.8.4` | `2.8.4` |
| License | `Apache-2.0 OR MIT` | `Apache-2.0 OR MIT` |
| `tauri` requirement | `^2.12` | `^2.12` (normal and build use; a dev use has the same range) |
| Normal dependency `rfd` | `^0.17` | `^0.17`, locked at `0.17.2` |
| Normal dependency `tauri-plugin-fs` | `^2.7.1` | `^2.7.1`, locked at `2.7.1` |
| Source | official Tauri org | registry `crates.io`, repository `https://github.com/tauri-apps/plugins-workspace` |
| `tauri` versions in the graph | one | one: `2.12.5` (`cargo tree -i tauri` lists `tauri v2.12.5`; `cargo tree -d` lists no `tauri`) |
| Pin | `=2.8.4` in the root `[workspace.dependencies]` | `tauri-plugin-dialog = "=2.8.4"`, next to `tauri = "=2.12.5"` |

`cargo tree --locked -p tauri-plugin-dialog -e normal --depth 1`:

```
tauri-plugin-dialog v2.8.4
├── log v0.4.35
├── raw-window-handle v0.6.2
├── rfd v0.17.2
├── serde v1.0.229
├── serde_json v1.0.154
├── tauri v2.12.5
├── tauri-plugin-fs v2.7.1
├── thiserror v2.0.21
└── url v2.5.8
```

### Crates the change adds to `Cargo.lock`

`git diff Cargo.lock` adds exactly three `[[package]]` entries and one dependency line on `office-harness-desktop`. Every other crate in the plugin's tree was already locked.

| Crate | Version | License (from `cargo metadata`) | With AGPL-3.0-or-later |
|---|---|---|---|
| `tauri-plugin-dialog` | 2.8.4 | `Apache-2.0 OR MIT` | compatible: both choices are permissive and one-way compatible |
| `tauri-plugin-fs` | 2.7.1 | `Apache-2.0 OR MIT` | compatible |
| `rfd` | 0.17.2 | `MIT` | compatible |

The webview gets no `dialog:` or `fs:` permission. `capabilities/default.json` holds only the `allow-*` entries of `WEBVIEW_COMMANDS` and `notification:default`; a Rust test in `commands.tests.rs` asserts this.
