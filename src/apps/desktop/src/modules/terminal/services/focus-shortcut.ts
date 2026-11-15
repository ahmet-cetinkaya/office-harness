// SPDX-License-Identifier: AGPL-3.0-or-later

import type { CoreClient } from "../../../core/api";
import {
  DEFAULT_FOCUS_SHORTCUT,
  parseShortcut,
  type Shortcut,
} from "../../../core/shortcuts";

const FOCUS_SHORTCUT_SETTING = "focus_shortcut";

export interface FocusShortcut {
  /** The setting's text, as shown to the user. */
  label: string;
  shortcut: Shortcut;
}

function fromText(text: string): FocusShortcut | null {
  const shortcut = parseShortcut(text);
  return shortcut === null ? null : { label: text, shortcut };
}

function defaultShortcut(): FocusShortcut {
  const parsed = fromText(DEFAULT_FOCUS_SHORTCUT);
  if (parsed === null) throw new Error("The default shortcut must parse.");
  return parsed;
}

/**
 * Reads the user's shortcut. The terminal must always have a way out, so an
 * unreadable or unparsable setting gives the default.
 */
export async function loadFocusShortcut(
  client: CoreClient,
): Promise<FocusShortcut> {
  try {
    const setting = (await client.sendRequest("settings.get", {
      key: FOCUS_SHORTCUT_SETTING,
    })) as { value?: unknown } | null | undefined;
    const text = typeof setting?.value === "string" ? setting.value : "";
    return fromText(text) ?? defaultShortcut();
  } catch {
    return defaultShortcut();
  }
}
