// SPDX-License-Identifier: AGPL-3.0-or-later

/** The most graphemes the row shows of a task text (FR-001). */
export const TASK_LINE_MAX_GRAPHEMES = 80;

const ELLIPSIS = "…";
const MS_PER_MINUTE = 60_000;
const MINUTES_PER_HOUR = 60;
const C0_END = 0x1f;
const DEL = 0x7f;
const C1_START = 0x80;
const C1_END = 0x9f;
const CONTROL_PICTURES_START = 0x2400;
const DEL_PICTURE = "␡";
const LINE_BREAK_CODE = 0x0a;
const LINE_BREAKS = /\r\n|\n|\r/;
const HEX_DIGITS = 4;

/**
 * Bidirectional format characters: they reorder the text around them, so a
 * row could show words in a different order than the agent was given.
 */
const BIDI_FORMAT_START_1 = 0x202a;
const BIDI_FORMAT_END_1 = 0x202e;
const BIDI_FORMAT_START_2 = 0x2066;
const BIDI_FORMAT_END_2 = 0x2069;

// \p{Cc} is C0, DEL, and C1.
const UNSAFE_CHARACTER = /[\p{Cc}\u202a-\u202e\u2066-\u2069]/gu;

const graphemes = new Intl.Segmenter(undefined, { granularity: "grapheme" });

function isBidiFormat(code: number): boolean {
  return (
    (code >= BIDI_FORMAT_START_1 && code <= BIDI_FORMAT_END_1) ||
    (code >= BIDI_FORMAT_START_2 && code <= BIDI_FORMAT_END_2)
  );
}

function escapeText(code: number): string {
  return `\\u${code.toString(16).toUpperCase().padStart(HEX_DIGITS, "0")}`;
}

function visibleCharacter(character: string): string {
  const code = character.charCodeAt(0);
  if (code === LINE_BREAK_CODE) return character;
  if (code <= C0_END) return String.fromCharCode(CONTROL_PICTURES_START + code);
  if (code === DEL) return DEL_PICTURE;
  if (code >= C1_START && code <= C1_END) return escapeText(code);
  return isBidiFormat(code) ? escapeText(code) : character;
}

/**
 * The text with every control character shown as a visible symbol, so that
 * nothing the agent or a user wrote is ever applied as a code (FR-006). A line
 * feed stays, because it only breaks the line.
 */
export function visibleText(text: string): string {
  return text.replace(UNSAFE_CHARACTER, visibleCharacter);
}

/** The first line of the text, at most 80 graphemes, in visible form. */
export function taskLine(text: string): string {
  const firstLine = text.split(LINE_BREAKS)[0];
  const parts = Array.from(
    graphemes.segment(firstLine),
    (part) => part.segment,
  );
  if (parts.length <= TASK_LINE_MAX_GRAPHEMES) return visibleText(firstLine);
  return (
    visibleText(parts.slice(0, TASK_LINE_MAX_GRAPHEMES).join("")) + ELLIPSIS
  );
}

/** "0 min ago" to "59 min ago", then "N h ago". A clock behind the server reads as 0. */
export function taskAge(setAt: number, now: number): string {
  const minutes = Math.max(0, Math.floor((now - setAt) / MS_PER_MINUTE));
  if (minutes < MINUTES_PER_HOUR) return `${minutes} min ago`;
  return `${Math.floor(minutes / MINUTES_PER_HOUR)} h ago`;
}

/** The row's task line, for example "Task given 3 min ago: Fix the login". */
export function taskRowText(
  task: { text: string; set_at: number },
  now: number,
) {
  return `Task given ${taskAge(task.set_at, now)}: ${taskLine(task.text)}`;
}
