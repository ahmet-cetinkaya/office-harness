// SPDX-License-Identifier: AGPL-3.0-or-later

/** The server's default for the `focus_shortcut` setting (A12). */
export const DEFAULT_FOCUS_SHORTCUT = "Ctrl+Alt+Escape";

const SEPARATOR = "+";
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
 * Reads text such as "Ctrl+Alt+Escape": modifiers first, the key last.
 * Returns null when the text names no key.
 */
export function parseShortcut(text: string): Shortcut | null {
  const parts = text.split(SEPARATOR).map((part) => part.trim().toLowerCase());
  const key = parts.pop();
  if (key === undefined || key === "") return null;
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
