// SPDX-License-Identifier: AGPL-3.0-or-later

import { tick } from "svelte";

/** The terminal view's focusable heading, which names the agent. */
const TERMINAL_HEADING_SELECTOR = "#terminal-heading";

/**
 * Where the jump leaves keyboard focus (0008 FR-007, D5): a running agent's
 * terminal takes focus by itself when it attaches; a read-only terminal of an
 * agent that is not running cannot take focus (0001), so focus goes to the
 * opened view's heading, which names the agent for a screen reader.
 */
export async function focusHeadingUnlessRunning(
  container: HTMLElement | null,
  isRunning: boolean,
): Promise<void> {
  if (isRunning) return;
  await tick();
  container?.querySelector<HTMLElement>(TERMINAL_HEADING_SELECTOR)?.focus();
}
