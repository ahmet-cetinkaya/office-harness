// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import type { CoreClient } from "../../../core/api";
import { DEFAULT_FOCUS_SHORTCUT } from "../../../core/shortcuts";
import { loadFocusShortcut } from "./focus-shortcut";

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
    const focus = await loadFocusShortcut(clientWithSetting("Ctrl+Shift+F11"));
    expect(focus.label).toBe("Ctrl+Shift+F11");
    expect(focus.shortcut.key).toBe("f11");
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
    const focus = await loadFocusShortcut(clientWithSetting("Ctrl+Shift+F1"));
    expect(focus.label).toBe(DEFAULT_FOCUS_SHORTCUT);
    expect(focus.shortcut.key).toBe("f12");
  });
});
