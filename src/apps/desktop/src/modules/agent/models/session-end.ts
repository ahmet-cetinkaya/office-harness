// SPDX-License-Identifier: AGPL-3.0-or-later

/** The text of a session that the server stopped uncleanly (0013). */
const LOST_TEXT = "lost: the server stopped uncleanly";
/** The server's form of a session that the office ended at a sign-in screen (0019). */
const NOT_SIGNED_IN_PREFIX = "not-signed-in: ";
/** The server's form of a session that never started. */
const NEVER_STARTED_PREFIX = "spawn-error: ";

/**
 * How a session ended, as text: the server's exit result with its machine
 * prefix replaced by words, so a sign-in reason reads as one sentence.
 */
export function sessionResultText(exitResult: string | null): string {
  if (exitResult === null) return "unknown";
  if (exitResult === "lost") return LOST_TEXT;
  if (exitResult.startsWith(NOT_SIGNED_IN_PREFIX)) {
    return `not signed in. ${exitResult.slice(NOT_SIGNED_IN_PREFIX.length)}`;
  }
  if (exitResult.startsWith(NEVER_STARTED_PREFIX)) {
    return `never started: ${exitResult.slice(NEVER_STARTED_PREFIX.length)}`;
  }
  return exitResult;
}
