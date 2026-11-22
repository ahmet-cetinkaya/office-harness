// SPDX-License-Identifier: AGPL-3.0-or-later

import { afterEach, describe, expect, test } from "bun:test";
import {
  APP_SHORTCUT_TABLE,
  DEFAULT_FOCUS_SHORTCUT,
  isAppShortcut,
  parseShortcut,
  isJumpUnbound,
  pressedAppShortcut,
  setCurrentFocusShortcut,
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
  test("holds twelve entries: agents 1 to 9, the switcher, the jump, the office hold", () => {
    expect(APP_SHORTCUT_TABLE).toHaveLength(12);
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
      null,
      null,
    ]);
    expect(APP_SHORTCUT_TABLE[9].label).toBe("Ctrl+Shift+F10");
    expect(APP_SHORTCUT_TABLE[10].label).toBe("Ctrl+Shift+F11");
    expect(APP_SHORTCUT_TABLE[10].kind).toBe("jump");
    expect(APP_SHORTCUT_TABLE[11].label).toBe("Ctrl+Shift+H");
  });

  test("holds twelve distinct combinations", () => {
    const keys = APP_SHORTCUT_TABLE.map((entry) =>
      JSON.stringify(entry.shortcut),
    );
    expect(new Set(keys).size).toBe(12);
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
    expect(pressedAppShortcut(pressed("Ctrl+Shift+F12"))).toBeNull();
    expect(pressedAppShortcut(pressed("Ctrl+Shift+F13"))).toBeNull();
  });

  test("each agent combination maps to its own position", () => {
    const positions = APP_SHORTCUT_TABLE.filter(
      (entry) => entry.position !== null,
    ).map((entry) => pressedAppShortcut(pressed(entry.label))?.position);
    expect(positions).toEqual([1, 2, 3, 4, 5, 6, 7, 8, 9]);
  });

  // The terminal lets out exactly the keys that pressedAppShortcut matches
  // (0008 modifies 0001/FR-017), so each kind must match from a terminal.
  test("every kind of entry leaves a focused terminal, the jump too", () => {
    expect(pressedAppShortcut(pressed("Ctrl+Shift+H"))?.kind).toBe(
      "office_hold",
    );
    expect(pressedAppShortcut(pressed("Ctrl+Shift+F1"))?.position).toBe(1);
    expect(pressedAppShortcut(pressed("Ctrl+Shift+F10"))?.kind).toBe(
      "switcher",
    );
    expect(pressedAppShortcut(pressed("Ctrl+Shift+F11"))?.kind).toBe("jump");
  });

  describe("when the focus shortcut holds the jump key (0008 D6)", () => {
    afterEach(() => setCurrentFocusShortcut(null));

    test("the jump entry is unbound and the other entries stay bound", () => {
      setCurrentFocusShortcut(parseShortcut("Shift+Ctrl+F11"));

      expect(isJumpUnbound()).toBe(true);
      expect(pressedAppShortcut(pressed("Ctrl+Shift+F11"))).toBeNull();
      expect(pressedAppShortcut(pressed("Ctrl+Shift+F10"))?.kind).toBe(
        "switcher",
      );
    });

    test("the jump is bound for the default focus shortcut", () => {
      setCurrentFocusShortcut(parseShortcut(DEFAULT_FOCUS_SHORTCUT));

      expect(isJumpUnbound()).toBe(false);
      expect(pressedAppShortcut(pressed("Ctrl+Shift+F11"))?.kind).toBe("jump");
    });
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
  // to "agent 9", "the agent switcher", the jump, and "the office hold"); nothing checks the server side
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
      "Ctrl+Shift+F11",
      "Ctrl+Shift+H",
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
    expect(isAppShortcut("Ctrl+Shift+F11")).toBe(true);
    expect(isAppShortcut("Ctrl+Shift+F13")).toBe(false);
    expect(isAppShortcut("Ctrl+F1")).toBe(false);
    expect(isAppShortcut("Ctrl+Shift")).toBe(false);
    expect(isAppShortcut("")).toBe(false);
  });
});
