// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import {
  APP_SHORTCUT_TABLE,
  DEFAULT_FOCUS_SHORTCUT,
  isAppShortcut,
  parseShortcut,
  pressedAppShortcut,
  type ShortcutEvent,
} from "./index";

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

function pressed(text: string): ShortcutEvent {
  const shortcut = parseShortcut(text);
  if (shortcut === null) throw new Error(`must parse: ${text}`);
  return keyEvent({
    key: shortcut.key.toUpperCase(),
    ctrlKey: shortcut.ctrl,
    altKey: shortcut.alt,
    shiftKey: shortcut.shift,
    metaKey: shortcut.meta,
  });
}

describe("app shortcut table", () => {
  test("holds ten entries: agents 1 to 9 and the switcher", () => {
    expect(APP_SHORTCUT_TABLE).toHaveLength(10);
    expect(APP_SHORTCUT_TABLE.map((entry) => entry.position)).toEqual([
      1,
      2,
      3,
      4,
      5,
      6,
      7,
      8,
      9,
      null,
    ]);
    expect(APP_SHORTCUT_TABLE[9].label).toBe("Ctrl+Shift+F10");
  });

  test("holds ten distinct combinations", () => {
    const keys = APP_SHORTCUT_TABLE.map((entry) =>
      JSON.stringify(entry.shortcut),
    );
    expect(new Set(keys).size).toBe(10);
  });

  test("no entry equals the focus shortcut", () => {
    const focus = parseShortcut(DEFAULT_FOCUS_SHORTCUT);
    expect(focus).not.toBeNull();
    const pressedFocus = keyEvent({
      key: "F12",
      ctrlKey: true,
      shiftKey: true,
    });
    for (const entry of APP_SHORTCUT_TABLE) {
      expect(entry.label).not.toBe(DEFAULT_FOCUS_SHORTCUT);
      // The comparison that matters is on key events, not on text.
      expect(pressedAppShortcut(pressedFocus)).toBeNull();
      expect(JSON.stringify(entry.shortcut)).not.toBe(JSON.stringify(focus));
    }
  });

  test("position 10 and above have no direct shortcut (A1)", () => {
    const positions = APP_SHORTCUT_TABLE.map((entry) => entry.position);
    expect(positions).not.toContain(10);
    expect(pressedAppShortcut(pressed("Ctrl+Shift+F10"))?.position).toBeNull();
    expect(pressedAppShortcut(pressed("Ctrl+Shift+F11"))).toBeNull();
    expect(pressedAppShortcut(pressed("Ctrl+Shift+F12"))).toBeNull();
  });

  test("each agent combination maps to its own position", () => {
    const positions = APP_SHORTCUT_TABLE.filter(
      (entry) => entry.position !== null,
    ).map((entry) => pressedAppShortcut(pressed(entry.label))?.position);
    expect(positions).toEqual([1, 2, 3, 4, 5, 6, 7, 8, 9]);
  });

  test("a key with the wrong modifiers matches nothing", () => {
    expect(pressedAppShortcut(keyEvent({ key: "F1" }))).toBeNull();
    expect(
      pressedAppShortcut(keyEvent({ key: "F1", ctrlKey: true })),
    ).toBeNull();
    expect(
      pressedAppShortcut(keyEvent({ key: "F1", shiftKey: true })),
    ).toBeNull();
    expect(pressedAppShortcut(keyEvent({ key: "F10" }))).toBeNull();
  });

  // SERVER_RESERVED mirrors settings.rs RESERVED_SHORTCUTS by hand ("agent 1"
  // to "agent 9" and "the agent switcher"); nothing checks the server side
  // automatically. This test pins the desktop table to that copy: a
  // desktop-side drift fails here, and a server-side change to
  // RESERVED_SHORTCUTS must be mirrored here in the same change, reviewed as
  // such.
  test("the table matches the combinations the server reserves", () => {
    const SERVER_RESERVED = [
      "Ctrl+Shift+F1",
      "Ctrl+Shift+F2",
      "Ctrl+Shift+F3",
      "Ctrl+Shift+F4",
      "Ctrl+Shift+F5",
      "Ctrl+Shift+F6",
      "Ctrl+Shift+F7",
      "Ctrl+Shift+F8",
      "Ctrl+Shift+F9",
      "Ctrl+Shift+F10",
    ];
    for (const label of SERVER_RESERVED)
      expect(isAppShortcut(label)).toBe(true);
    expect(APP_SHORTCUT_TABLE.map((entry) => entry.label)).toEqual(
      SERVER_RESERVED,
    );
    expect(isAppShortcut(DEFAULT_FOCUS_SHORTCUT)).toBe(false);
  });

  test("isAppShortcut compares combinations, not spellings", () => {
    expect(isAppShortcut("ctrl + shift + f1")).toBe(true);
    expect(isAppShortcut("Shift+Ctrl+F1")).toBe(true);
    expect(isAppShortcut("Ctrl+Shift+F12")).toBe(false);
    expect(isAppShortcut("Ctrl+Shift+F11")).toBe(false);
    expect(isAppShortcut("Ctrl+F1")).toBe(false);
    expect(isAppShortcut("Ctrl+Shift")).toBe(false);
    expect(isAppShortcut("")).toBe(false);
  });
});
