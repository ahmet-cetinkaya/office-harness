// SPDX-License-Identifier: AGPL-3.0-or-later

import {
  matchesShortcut,
  parseShortcut,
  type Shortcut,
  type ShortcutEvent,
} from "./shortcut";

/** One entry of the app's key-binding table (0003 D8). */
export interface AppShortcut {
  /** The combination as shown to the user, such as "Ctrl+Shift+F1". */
  label: string;
  /** The agent position of a direct shortcut, or null for the switcher. */
  position: number | null;
  shortcut: Shortcut;
}

/** The modifier family of the focus shortcut, which the table shares (A1). */
const MODIFIERS = "Ctrl+Shift";
/** The key that opens the agent at `index + 1` of the list (A1: the first nine). */
const AGENT_KEYS = ["F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9"];

function bound(label: string, position: number | null): AppShortcut {
  const shortcut = parseShortcut(label);
  if (shortcut === null) {
    throw new Error(`The shortcut default must parse: ${label}`);
  }
  return { label, position, shortcut };
}

/** The table: the first nine agents and the switcher, ten combinations (D8). */
export const APP_SHORTCUT_TABLE: AppShortcut[] = [
  ...AGENT_KEYS.map((key, index) => bound(`${MODIFIERS}+${key}`, index + 1)),
  bound(`${MODIFIERS}+F10`, null),
];

/** The table entry the key event presses, or null for any other key (D8). */
export function pressedAppShortcut(event: ShortcutEvent): AppShortcut | null {
  return (
    APP_SHORTCUT_TABLE.find((entry) =>
      matchesShortcut(entry.shortcut, event),
    ) ?? null
  );
}
