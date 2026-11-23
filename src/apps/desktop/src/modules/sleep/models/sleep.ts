// SPDX-License-Identifier: AGPL-3.0-or-later

/** The agent status of a slept agent (contract: sleep protocol). */
export const ASLEEP_STATUS = "asleep";

/** The setting that holds the idle time in minutes; "0" is off (FR-001). */
export const IDLE_SLEEP_KEY = "idle_sleep_minutes";
export const IDLE_SLEEP_OFF = "0";
/** What the minutes field shows when the person turns idle sleep on. */
export const DEFAULT_IDLE_MINUTES = "30";
export const IDLE_MINUTES_MIN = 5;
export const IDLE_MINUTES_MAX = 1440;
export const IDLE_MINUTES_INVALID_MESSAGE =
  "idle_sleep_minutes is 0 (off) or a whole number of minutes from 5 to 1440";

export const ASLEEP_TEXT =
  "Asleep. Press Enter or Wake to continue the last session.";
export const NO_LONGER_ASLEEP_TEXT = "This agent is no longer asleep";
export const WAKING_TEXT = "Waking";
export const IDLE_SLEEP_HELP_TEXT =
  "The office ends the CLI of an agent that stays idle this long, keeps its session, and continues it on Wake. A CLI that cannot continue a session starts a new one. The sleep also ends every program that the CLI started and that still runs, such as a background task or a development server.";

const WHOLE_NUMBER = /^\d+$/;

export type WakeResultKind = "checking" | "continued" | "new_session" | "held";

/** One `wake.result` event. */
export interface WakeResult {
  agent_id: string;
  result: WakeResultKind;
  continued: boolean;
  reason: string | null;
  isFinal: boolean;
}

/** What the person sees of one agent's wake, newest only. */
export type WakeEntry =
  | { kind: "pending" }
  | { kind: "result"; result: WakeResult }
  | { kind: "failure"; text: string };

const RESULT_KINDS: readonly string[] = [
  "checking",
  "continued",
  "new_session",
  "held",
];

/** The setting as the dialog shows it: the switch, and the minutes text. */
export interface IdleSleepForm {
  isOn: boolean;
  minutes: string;
}

/** The form for a stored setting value; off shows the default minutes. */
export function readIdleSleepForm(value: string): IdleSleepForm {
  const isOn = value !== "" && value !== IDLE_SLEEP_OFF;
  return { isOn, minutes: isOn ? value : DEFAULT_IDLE_MINUTES };
}

/** The message that refuses the text as an idle time, or null when it is one. */
export function minutesProblem(text: string): string | null {
  const trimmed = text.trim();
  if (!WHOLE_NUMBER.test(trimmed)) return IDLE_MINUTES_INVALID_MESSAGE;
  const value = Number(trimmed);
  return value >= IDLE_MINUTES_MIN && value <= IDLE_MINUTES_MAX
    ? null
    : IDLE_MINUTES_INVALID_MESSAGE;
}

/** The refusal of the form, or null when it can be saved. Off needs no minutes. */
export function formProblem(form: IdleSleepForm): string | null {
  return form.isOn ? minutesProblem(form.minutes) : null;
}

/** The setting value that the form stores. */
export function settingValueOf(form: IdleSleepForm): string {
  return form.isOn ? form.minutes.trim() : IDLE_SLEEP_OFF;
}

function withReason(lead: string, reason: string | null): string {
  return reason === null ? lead : `${lead}: ${reason}`;
}

/** The `wake.result` event as a result, or null when it is not one. */
export function readWakeResult(event: unknown): WakeResult | null {
  if (typeof event !== "object" || event === null) return null;
  const fields = event as Record<string, unknown>;
  const { agent_id: agentId, result } = fields;
  if (typeof agentId !== "string" || typeof result !== "string") return null;
  if (!RESULT_KINDS.includes(result)) return null;
  return {
    agent_id: agentId,
    result: result as WakeResultKind,
    continued: fields.continued === true,
    reason: typeof fields.reason === "string" ? fields.reason : null,
    isFinal: fields.is_final === true,
  };
}

/** One wake entry as text; never colour alone (FR-008). */
export function wakeText(entry: WakeEntry): string {
  if (entry.kind === "pending") return WAKING_TEXT;
  if (entry.kind === "failure") return entry.text;
  switch (entry.result.result) {
    case "checking":
      return "Checking";
    case "continued":
      return "Continued";
    case "new_session":
      return withReason("New session", entry.result.reason);
    case "held":
      return withReason("On hold", entry.result.reason);
  }
}

/** True while a wake runs, so Wake is not available (FR-008). */
export function isWakeBusy(entry: WakeEntry | undefined): boolean {
  if (entry === undefined) return false;
  return (
    entry.kind === "pending" ||
    (entry.kind === "result" && entry.result.result === "checking")
  );
}
