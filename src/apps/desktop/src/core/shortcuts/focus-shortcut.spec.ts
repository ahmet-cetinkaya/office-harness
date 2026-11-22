// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import type { CoreClient } from "../api";
import { loadFocusShortcut } from "./focus-shortcut";
import { DEFAULT_FOCUS_SHORTCUT } from "./shortcut";

function clientWithSetting(value: unknown): CoreClient {
  return {
    watchConnection: async () => {},
    watchEvents: async () => {},
    watchTerminalFrames: async () => {},
    sendRequest: async (kind) => {
      if (kind === "settings.get") return { value };
      return null;
    },
    startServer: async () => {},
    registerLoginStart: async () => {},
    loginStartStatus: async () => false,
    retryConnection: async () => {},
  };
}

describe("focus shortcut", () => {
  test("a stored value names a shortcut that resolves", async () => {
    const focus = await loadFocusShortcut(clientWithSetting("Ctrl+Alt+F11"));
    expect(focus.label).toBe("Ctrl+Alt+F11");
    expect(focus.shortcut.key).toBe("f11");
  });

  // The jump key is the one table key that a user may hold from before 0008
  // (D6): the focus shortcut keeps it and the jump has no key.
  test("a stored jump key is kept, in every spelling", async () => {
    for (const stored of ["Ctrl+Shift+F11", "Shift+Ctrl+F11"]) {
      const focus = await loadFocusShortcut(clientWithSetting(stored));
      expect(focus.label).toBe(stored);
      expect(focus.shortcut.key).toBe("f11");
    }
  });

  test("an unparsable stored value gives the default", async () => {
    const focus = await loadFocusShortcut(clientWithSetting("Ctrl"));
    expect(focus.label).toBe(DEFAULT_FOCUS_SHORTCUT);
  });

  test("a missing stored value gives the default", async () => {
    const focus = await loadFocusShortcut(clientWithSetting(undefined));
    expect(focus.label).toBe(DEFAULT_FOCUS_SHORTCUT);
  });

  // The server's settings validator refuses a focus_shortcut that names an
  // app-table combination, but a stale stored value could be one; it must
  // not eat the table's keys, so it resolves to the default instead.
  test("a stored app-table combination gives the default", async () => {
    for (const stored of ["Ctrl+Shift+F1", "Ctrl+Shift+F10", "Ctrl+Shift+H"]) {
      const focus = await loadFocusShortcut(clientWithSetting(stored));
      expect(focus.label).toBe(DEFAULT_FOCUS_SHORTCUT);
      expect(focus.shortcut.key).toBe("f12");
    }
  });
});
