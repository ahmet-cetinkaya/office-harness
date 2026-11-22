// SPDX-License-Identifier: AGPL-3.0-or-later

import { currentFocusShortcut } from "./focus-shortcut-state";
import {
  matchesShortcut,
  parseShortcut,
  sameShortcut,
  type Shortcut,
  type ShortcutEvent,
} from "./shortcut";

/**
 * One entry of the app's key-binding table (0003 D8). Every entry leaves a
 * focused terminal for the app (0008 modifies 0001/FR-017), the office hold
 * and the jump included.
 */
export interface AppShortcut {
  /** The combination as shown to the user, such as "Ctrl+Shift+F1". */
  label: string;
  /** What the key does in the panel. */
  kind: "agent" | "switcher" | "jump" | "office_hold";
  /** The agent position of a direct shortcut, or null for any other kind. */
  position: number | null;
  shortcut: Shortcut;
}

/** The modifier family of the focus shortcut, which the table shares (A1). */
const MODIFIERS = "Ctrl+Shift";
/** The key that opens the agent at `index + 1` of the list (A1: the first nine). */
const AGENT_KEYS = ["F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9"];

function bound(
  label: string,
  kind: AppShortcut["kind"],
  position: number | null,
): AppShortcut {
  const shortcut = parseShortcut(label);
  if (shortcut === null) {
    throw new Error(`The shortcut default must parse: ${label}`);
  }
  return { label, kind, position, shortcut };
}

/**
 * The table: the first nine agents, the switcher, the jump to the agent at
 * the top of Needs you, and the office hold, twelve combinations (0003 D8,
 * 0007 D9, 0008 D5). The order is the server's `RESERVED_SHORTCUTS`.
 */
export const APP_SHORTCUT_TABLE: AppShortcut[] = [
  ...AGENT_KEYS.map((key, index) =>
    bound(`${MODIFIERS}+${key}`, "agent", index + 1),
  ),
  bound(`${MODIFIERS}+F10`, "switcher", null),
  bound(`${MODIFIERS}+F11`, "jump", null),
  bound(`${MODIFIERS}+H`, "office_hold", null),
];

/**
 * True while the user's focus shortcut holds the jump key: the focus
 * shortcut keeps it and the jump has no key (0008 D6).
 */
function isUnbound(entry: AppShortcut): boolean {
  const focus = currentFocusShortcut();
  return (
    entry.kind === "jump" &&
    focus !== null &&
    sameShortcut(entry.shortcut, focus)
  );
}

/** True while the focus shortcut holds the jump key (0008 D6). */
export function isJumpUnbound(): boolean {
  return APP_SHORTCUT_TABLE.some(isUnbound);
}

/** The table entry the key event presses, or null for any other key (D8). */
export function pressedAppShortcut(event: ShortcutEvent): AppShortcut | null {
  return (
    APP_SHORTCUT_TABLE.find(
      (entry) => matchesShortcut(entry.shortcut, event) && !isUnbound(entry),
    ) ?? null
  );
}

/**
 * The table entry whose combination the text names, compared as the
 * server's settings validator compares (settings.rs `normalized_shortcut`),
 * or null. It ignores the unbound jump, because it answers what the table
 * reserves, not what is bound right now.
 */
export function appShortcutNamed(text: string): AppShortcut | null {
  const shortcut = parseShortcut(text);
  if (shortcut === null) return null;
  return (
    APP_SHORTCUT_TABLE.find((entry) =>
      sameShortcut(entry.shortcut, shortcut),
    ) ?? null
  );
}
