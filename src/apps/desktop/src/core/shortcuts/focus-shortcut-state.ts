// SPDX-License-Identifier: AGPL-3.0-or-later

import type { Shortcut } from "./shortcut";

/**
 * The one copy of the user's focus shortcut, which the table needs to leave
 * the jump key unbound when the focus shortcut holds it (0008 D6). Kept
 * outside the loader so that the table does not import the loader.
 */
let current: Shortcut | null = null;

export function setCurrentFocusShortcut(shortcut: Shortcut | null): void {
  current = shortcut;
}

export function currentFocusShortcut(): Shortcut | null {
  return current;
}
