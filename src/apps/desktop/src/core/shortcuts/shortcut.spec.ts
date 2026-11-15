// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import {
  DEFAULT_FOCUS_SHORTCUT,
  matchesShortcut,
  parseShortcut,
  type ShortcutEvent,
} from "./shortcut";

function keyEvent(overrides: Partial<ShortcutEvent>): ShortcutEvent {
  return {
    key: "a",
    ctrlKey: false,
    altKey: false,
    shiftKey: false,
    metaKey: false,
    ...overrides,
  };
}

describe("parseShortcut", () => {
  test("reads modifiers and the key, ignoring case", () => {
    expect(parseShortcut("ctrl+SHIFT+K")).toEqual({
      ctrl: true,
      alt: false,
      shift: true,
      meta: false,
      key: "k",
    });
  });

  test("knows the short names Esc and Space", () => {
    expect(parseShortcut("Alt+Esc")?.key).toBe("escape");
    expect(parseShortcut("Ctrl+Space")?.key).toBe(" ");
  });

  test("refuses text without a key", () => {
    expect(parseShortcut("")).toBeNull();
    expect(parseShortcut("Ctrl+")).toBeNull();
  });
});

describe("matchesShortcut", () => {
  const shortcut = parseShortcut(DEFAULT_FOCUS_SHORTCUT);

  test("matches the default only with its exact modifiers", () => {
    expect(shortcut).not.toBeNull();
    if (shortcut === null) return;
    const pressed = keyEvent({ key: "Escape", ctrlKey: true, altKey: true });
    expect(matchesShortcut(shortcut, pressed)).toBe(true);
    expect(matchesShortcut(shortcut, { ...pressed, shiftKey: true })).toBe(
      false,
    );
    expect(matchesShortcut(shortcut, { ...pressed, altKey: false })).toBe(
      false,
    );
  });

  test("does not match other keys, including a bare Escape and Ctrl+C", () => {
    if (shortcut === null) throw new Error("the default must parse");
    expect(matchesShortcut(shortcut, keyEvent({ key: "Escape" }))).toBe(false);
    expect(
      matchesShortcut(shortcut, keyEvent({ key: "c", ctrlKey: true })),
    ).toBe(false);
  });
});
