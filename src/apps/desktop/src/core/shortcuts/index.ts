// SPDX-License-Identifier: AGPL-3.0-or-later

export {
  APP_SHORTCUT_TABLE,
  isJumpUnbound,
  pressedAppShortcut,
  type AppShortcut,
} from "./app-shortcuts";
export {
  currentFocusShortcut,
  setCurrentFocusShortcut,
} from "./focus-shortcut-state";
export {
  loadFocusShortcut,
  sharedFocusShortcut,
  type FocusShortcut,
} from "./focus-shortcut";
export {
  DEFAULT_FOCUS_SHORTCUT,
  matchesShortcut,
  parseShortcut,
  sameShortcut,
  type Shortcut,
  type ShortcutEvent,
} from "./shortcut";
