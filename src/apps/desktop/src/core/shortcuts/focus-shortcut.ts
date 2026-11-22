// SPDX-License-Identifier: AGPL-3.0-or-later

import type { CoreClient } from "../api";
import { appShortcutNamed } from "./app-shortcuts";
import { setCurrentFocusShortcut } from "./focus-shortcut-state";
import {
  DEFAULT_FOCUS_SHORTCUT,
  parseShortcut,
  type Shortcut,
} from "./shortcut";

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
 * True when a stored text names a table key that the focus shortcut must
 * give up. The jump key is the exception: a user may hold it from before
 * 0008, and the jump has no key then (0008 D6).
 */
function takesTableKey(text: string): boolean {
  const named = appShortcutNamed(text);
  return named !== null && named.kind !== "jump";
}

/**
 * Reads the user's shortcut. The terminal must always have a way out, so an
 * unparsable setting gives the default. A failed read rejects, so that the
 * caller can tell the default it gets from the one the user stored.
 */
export async function loadFocusShortcut(
  client: CoreClient,
): Promise<FocusShortcut> {
  const setting = (await client.sendRequest("settings.get", {
    key: FOCUS_SHORTCUT_SETTING,
  })) as { value?: unknown } | null | undefined;
  const text = typeof setting?.value === "string" ? setting.value : "";
  // A stored value the server's validator would refuse (an app-table
  // combination) must not eat the table's keys; the terminal keeps its
  // default way out.
  if (takesTableKey(text)) return defaultShortcut();
  return fromText(text) ?? defaultShortcut();
}

let shared: Promise<FocusShortcut> | null = null;

/**
 * The focus shortcut, loaded once for the whole app (0008 D6): the terminal,
 * the panel, the scene, and the Needs you section all read this one copy. A
 * change of the setting applies at the next app start. A failed load is not
 * kept: the caller gets the default for now and the next call reads again,
 * so one failed read cannot replace the user's stored key for good.
 */
export function sharedFocusShortcut(
  client: CoreClient,
): Promise<FocusShortcut> {
  if (shared === null) {
    const attempt = loadFocusShortcut(client).then((focus) => {
      setCurrentFocusShortcut(focus.shortcut);
      return focus;
    });
    shared = attempt;
    attempt.catch(() => {
      if (shared === attempt) shared = null;
    });
  }
  return shared.catch(defaultShortcut);
}
