// SPDX-License-Identifier: AGPL-3.0-or-later

/** The server's default for the `focus_shortcut` setting (A12). */
export const DEFAULT_FOCUS_SHORTCUT = "Ctrl+Shift+F12";

const SEPARATOR = "+";
const MODIFIER_NAMES = ["ctrl", "control", "alt", "shift", "meta"];
const KEY_ALIASES: Record<string, string> = { esc: "escape", space: " " };

export interface Shortcut {
  ctrl: boolean;
  alt: boolean;
  shift: boolean;
  meta: boolean;
  /** The lower-case `KeyboardEvent.key` value. */
  key: string;
}

export type ShortcutEvent = Pick<
  KeyboardEvent,
  "key" | "ctrlKey" | "altKey" | "shiftKey" | "metaKey"
>;

/**
 * Reads text such as "Ctrl+Shift+F12": modifiers first, the key last.
 * Returns null when the text names no key, or only a modifier.
 */
export function parseShortcut(text: string): Shortcut | null {
  const parts = text.split(SEPARATOR).map((part) => part.trim().toLowerCase());
  const key = parts.pop();
  if (key === undefined || key === "" || MODIFIER_NAMES.includes(key)) {
    return null;
  }
  return {
    ctrl: parts.includes("ctrl"),
    alt: parts.includes("alt"),
    shift: parts.includes("shift"),
    meta: parts.includes("meta"),
    key: KEY_ALIASES[key] ?? key,
  };
}

/** True when the event has exactly the shortcut's modifiers and key. */
export function matchesShortcut(
  shortcut: Shortcut,
  event: ShortcutEvent,
): boolean {
  return (
    event.key.toLowerCase() === shortcut.key &&
    event.ctrlKey === shortcut.ctrl &&
    event.altKey === shortcut.alt &&
    event.shiftKey === shortcut.shift &&
    event.metaKey === shortcut.meta
  );
}

/** True when both name the same modifiers and key. */
export function sameShortcut(first: Shortcut, second: Shortcut): boolean {
  return (
    first.key === second.key &&
    first.ctrl === second.ctrl &&
    first.alt === second.alt &&
    first.shift === second.shift &&
    first.meta === second.meta
  );
}
