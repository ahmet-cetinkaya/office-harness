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
  /** What the key does in the panel. */
  kind: "agent" | "switcher" | "office_hold";
  /** The agent position of a direct shortcut, or null for any other kind. */
  position: number | null;
  /**
   * True when the key leaves a focused terminal for the app (0003 D8). The
   * office hold is one too, so it holds from a terminal (0007 D9 = b).
   */
  isTerminalShortcut: boolean;
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
  isTerminalShortcut: boolean,
): AppShortcut {
  const shortcut = parseShortcut(label);
  if (shortcut === null) {
    throw new Error(`The shortcut default must parse: ${label}`);
  }
  return { label, kind, position, isTerminalShortcut, shortcut };
}

/**
 * The table: the first nine agents, the switcher, and the office hold,
 * eleven combinations (0003 D8, 0007 D9).
 */
export const APP_SHORTCUT_TABLE: AppShortcut[] = [
  ...AGENT_KEYS.map((key, index) =>
    bound(`${MODIFIERS}+${key}`, "agent", index + 1, true),
  ),
  bound(`${MODIFIERS}+F10`, "switcher", null, true),
  bound(`${MODIFIERS}+H`, "office_hold", null, true),
];

/** The table entry the key event presses, or null for any other key (D8). */
export function pressedAppShortcut(event: ShortcutEvent): AppShortcut | null {
  return (
    APP_SHORTCUT_TABLE.find((entry) =>
      matchesShortcut(entry.shortcut, event),
    ) ?? null
  );
}

/** The table entry that a focused terminal lets out of xterm, or null (D8). */
export function pressedTerminalShortcut(
  event: ShortcutEvent,
): AppShortcut | null {
  const pressed = pressedAppShortcut(event);
  return pressed?.isTerminalShortcut ? pressed : null;
}

/**
 * True when the text names a combination of the table, compared as the
 * server's settings validator compares (settings.rs `normalized_shortcut`):
 * the focus shortcut must not take any of them, so this is the one list the
 * desktop checks a focus shortcut against.
 */
export function isAppShortcut(text: string): boolean {
  const shortcut = parseShortcut(text);
  if (shortcut === null) return false;
  return (
    pressedAppShortcut({
      key: shortcut.key,
      ctrlKey: shortcut.ctrl,
      altKey: shortcut.alt,
      shiftKey: shortcut.shift,
      metaKey: shortcut.meta,
    }) !== null
  );
}
