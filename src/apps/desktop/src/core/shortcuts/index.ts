// SPDX-License-Identifier: AGPL-3.0-or-later

export {
  APP_SHORTCUT_TABLE,
  isAppShortcut,
  pressedAppShortcut,
  pressedTerminalShortcut,
  isHeldForTerminal,
  type KeyTarget,
  type AppShortcut,
} from "./app-shortcuts";
export {
  DEFAULT_FOCUS_SHORTCUT,
  matchesShortcut,
  parseShortcut,
  type Shortcut,
  type ShortcutEvent,
} from "./shortcut";
