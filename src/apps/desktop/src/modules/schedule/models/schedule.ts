// SPDX-License-Identifier: AGPL-3.0-or-later

import type { FieldFailure } from "../../../core/api";

export type RepeatKind = "once" | "daily" | "weekly" | "every_hours";
export type WeekdayCode = "mon" | "tue" | "wed" | "thu" | "fri" | "sat" | "sun";

/** The repeat of a schedule, as the wire carries it (0025 contract). */
export interface Repeat {
  kind: RepeatKind;
  /** "YYYY-MM-DD", for a once repeat. */
  date?: string;
  /** "HH:MM" in the server's zone, for a once, daily, or weekly repeat. */
  time?: string;
  days?: WeekdayCode[];
  hours?: number;
}

export type OffReason = "target_deleted" | "time_missed";

/** One schedule, as `schedule.list` and `schedule.changed` answer it. */
export interface Schedule {
  id: string;
  name: string;
  text: string;
  target_agent_id: string;
  repeat: Repeat;
  on: boolean;
  /** Unix milliseconds; null while the schedule is off or done. */
  next_due: number | null;
  last_handled_due: number | null;
  last_missed: number | null;
  off_reason: OffReason | null;
}

/** One row of a schedule's history, as `schedule.history` answers it. */
export interface RunRecord {
  id: number;
  schedule_id: string;
  due: number;
  started: number;
  /** "due" for a run at its time, "now" for Run now. */
  via: "due" | "now";
  /** Null while the run is open. */
  result: string | null;
  reason: string | null;
}

export interface ScheduleList {
  schedules: Schedule[];
  /** The zone the server computes due times in. */
  zone: string;
}

export const EMPTY_SCHEDULE_LIST: ScheduleList = { schedules: [], zone: "" };

export const NO_SCHEDULES_TEXT = "No schedules.";
export const NO_HISTORY_TEXT = "This schedule has not run yet.";
export const NEVER_RAN_TEXT = "never ran";
export const NOT_SCHEDULED_TEXT = "none";
export const HISTORY_SHOWN = 20;

export const WEEKDAYS: readonly { code: WeekdayCode; label: string }[] = [
  { code: "mon", label: "Monday" },
  { code: "tue", label: "Tuesday" },
  { code: "wed", label: "Wednesday" },
  { code: "thu", label: "Thursday" },
  { code: "fri", label: "Friday" },
  { code: "sat", label: "Saturday" },
  { code: "sun", label: "Sunday" },
];

export const REPEAT_KINDS: readonly { value: RepeatKind; label: string }[] = [
  { value: "once", label: "Once" },
  { value: "daily", label: "Every day" },
  { value: "weekly", label: "On chosen days of the week" },
  { value: "every_hours", label: "Every N hours" },
];

const OFF_REASON_TEXT: Record<OffReason, string> = {
  target_deleted:
    "its target agent was deleted; edit it to name another target",
  time_missed:
    "its time was missed while the office was off; edit it to name a new time",
};

const RESULT_TEXT: Record<string, string> = {
  submitted: "Submitted",
  waiting: "Waiting for the agent",
  unsubmitted: "Not submitted",
  withdrawn: "Withdrawn",
  refused: "Refused",
  skipped: "Skipped",
  failed: "Failed",
  missed: "Missed",
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

/** The `schedule.list` answer; a malformed answer reads as an empty list. */
export function readScheduleList(result: unknown): ScheduleList {
  if (!isRecord(result)) return EMPTY_SCHEDULE_LIST;
  return {
    schedules: Array.isArray(result.schedules)
      ? (result.schedules as Schedule[])
      : [],
    zone: typeof result.zone === "string" ? result.zone : "",
  };
}

/** The schedule inside an answer or an event, or null when there is none. */
export function readSchedule(value: unknown): Schedule | null {
  if (!isRecord(value) || typeof value.id !== "string") return null;
  return value as unknown as Schedule;
}

/** The `schedule.history` answer, newest first, at most the shown count. */
export function readHistory(result: unknown): RunRecord[] {
  if (!isRecord(result) || !Array.isArray(result.runs)) return [];
  return (result.runs as RunRecord[]).slice(0, HISTORY_SHOWN);
}

/** A time of the day for a locale-neutral reading, "09:00". */
function timeText(time: string | undefined): string {
  return time ?? "";
}

/** "Mon, 24 Nov 2025" for a "YYYY-MM-DD" date; the date itself when it is none. */
export function dateText(date: string, locale?: string): string {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(date);
  if (match === null) return date;
  const [, year, month, day] = match;
  const moment = new Date(
    Date.UTC(Number(year), Number(month) - 1, Number(day)),
  );
  return new Intl.DateTimeFormat(locale, {
    weekday: "short",
    year: "numeric",
    month: "short",
    day: "numeric",
    timeZone: "UTC",
  }).format(moment);
}

/** The repeat in words: "every day at 09:00", "every 6 hours". */
export function repeatWords(repeat: Repeat, locale?: string): string {
  switch (repeat.kind) {
    case "once":
      return `once on ${dateText(repeat.date ?? "", locale)} at ${timeText(repeat.time)}`;
    case "daily":
      return `every day at ${timeText(repeat.time)}`;
    case "weekly": {
      const names = WEEKDAYS.filter((day) =>
        (repeat.days ?? []).includes(day.code),
      ).map((day) => day.label);
      return `every ${names.join(", ")} at ${timeText(repeat.time)}`;
    }
    case "every_hours": {
      const hours = repeat.hours ?? 0;
      return hours === 1 ? "every hour" : `every ${hours} hours`;
    }
  }
}

/**
 * A due time as weekday, date, and time in the machine's zone, or in the
 * given zone (the tests use it); null reads as "none" (0025 D20).
 */
export function dueText(
  millis: number | null,
  timeZone?: string,
  locale?: string,
): string {
  if (millis === null) return NOT_SCHEDULED_TEXT;
  return new Intl.DateTimeFormat(locale, {
    weekday: "short",
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
    timeZone,
  }).format(new Date(millis));
}

export function switchText(schedule: Schedule): "on" | "off" {
  return schedule.on ? "on" : "off";
}

/** Why the office switched a schedule off, as a sentence, or null. */
export function offReasonText(schedule: Schedule): string | null {
  if (schedule.off_reason === null) return null;
  return `Switched off by the office: ${OFF_REASON_TEXT[schedule.off_reason]}.`;
}

export function resultText(result: string | null): string {
  if (result === null) return "Running";
  return RESULT_TEXT[result] ?? result.replaceAll("_", " ");
}

/** The last run of a schedule: its result and its due time. */
export function lastRunText(last: RunRecord | null, timeZone?: string): string {
  if (last === null) return NEVER_RAN_TEXT;
  return `${resultText(last.result)}, due ${dueText(last.due, timeZone)}`;
}

/** The accessible name of a row: name, switch, and next run (0025 D19). */
export function rowName(schedule: Schedule, timeZone?: string): string {
  return `${schedule.name}, ${switchText(schedule)}, next run ${dueText(schedule.next_due, timeZone)}`;
}

/** The accessible description of a row: action, repeat, and last result. */
export function rowDescription(
  schedule: Schedule,
  targetName: string,
  last: RunRecord | null,
  timeZone?: string,
): string {
  return `Sends "${schedule.text}" to ${targetName}, ${repeatWords(schedule.repeat)}. Last run: ${lastRunText(last, timeZone)}.`;
}

/** The via of a history row as words. */
export function viaText(via: RunRecord["via"]): string {
  return via === "now" ? "Run now" : "Due";
}

// ---- the form ------------------------------------------------------------

export const NAME_MAX_CHARACTERS = 100;
export const TEXT_MAX_CHARACTERS = 8192;
export const MIN_HOURS = 1;
export const MAX_HOURS = 168;

export type FieldName = "name" | "text" | "repeat" | "target";

export const FIELD_LABELS: Record<FieldName, string> = {
  name: "Name",
  text: "Text",
  repeat: "Repeat",
  target: "Target",
};

/** The values of the form, as the person typed them. */
export interface ScheduleDraft {
  name: string;
  text: string;
  targetAgentId: string;
  kind: RepeatKind;
  date: string;
  time: string;
  days: WeekdayCode[];
  hours: string;
  on: boolean;
}

export function emptyDraft(targetAgentId: string): ScheduleDraft {
  return {
    name: "",
    text: "",
    targetAgentId,
    kind: "daily",
    date: "",
    time: "09:00",
    days: [],
    hours: "6",
    on: true,
  };
}

export function draftOf(schedule: Schedule): ScheduleDraft {
  const { repeat } = schedule;
  return {
    name: schedule.name,
    text: schedule.text,
    targetAgentId: schedule.target_agent_id,
    kind: repeat.kind,
    date: repeat.date ?? "",
    time: repeat.time ?? "09:00",
    days: [...(repeat.days ?? [])],
    hours: String(repeat.hours ?? 6),
    on: schedule.on,
  };
}

function repeatOf(draft: ScheduleDraft): Repeat {
  switch (draft.kind) {
    case "once":
      return { kind: "once", date: draft.date, time: draft.time };
    case "daily":
      return { kind: "daily", time: draft.time };
    case "weekly":
      return { kind: "weekly", days: [...draft.days], time: draft.time };
    case "every_hours":
      return { kind: "every_hours", hours: Number(draft.hours) };
  }
}

/** The fields of `schedule.save`; an edit names its schedule. */
export function saveFields(
  draft: ScheduleDraft,
  scheduleId: string | null,
): Record<string, unknown> {
  return {
    ...(scheduleId === null ? {} : { schedule_id: scheduleId }),
    name: draft.name.trim(),
    text: draft.text,
    target_agent_id: draft.targetAgentId,
    repeat: repeatOf(draft),
    on: draft.on,
  };
}

/** One failing field and the rule it broke, as the server words them. */
export type FieldProblem = FieldFailure;

const RULE_MESSAGES = {
  blank: "must not be blank",
  control_character: "must not have control characters",
  format_character: "must not have invisible or direction-changing characters",
  no_day: "a weekly repeat needs at least one day",
  hours_out_of_range: `hours must be a whole number from ${MIN_HOURS} to ${MAX_HOURS}`,
  bad_time: "a time must be hours and minutes, HH:MM",
  bad_date: "a date must be a real calendar date, YYYY-MM-DD",
  target_missing: "must name an agent that exists",
} as const;

function problem(
  field: FieldName,
  rule: keyof typeof RULE_MESSAGES | "too_long",
  max = 0,
): FieldProblem {
  const message =
    rule === "too_long"
      ? `must have at most ${max} characters`
      : RULE_MESSAGES[rule];
  return { field, rule, message };
}

// A line break counts as a control character, as in a directive's text.
const CONTROL_CHARACTER = /\p{Cc}/u;
// The names the server refuses (0025 follow-up): zero-width and bidi format
// characters. A directive's text may keep them (an emoji joiner).
const FORMAT_CHARACTER =
  /[\u00AD\u0600-\u0605\u061C\u06DD\u070F\u180E\u200B-\u200F\u202A-\u202E\u2060-\u2064\u2066-\u206F\uFEFF\uFFF9-\uFFFB\u{E0001}\u{E0020}-\u{E007F}]/u;
const TIME_PATTERN = /^([01]\d|2[0-3]):[0-5]\d$/;
const DATE_PATTERN = /^\d{4}-\d{2}-\d{2}$/;
const WHOLE_NUMBER = /^\d+$/;

function textProblem(
  field: "name" | "text",
  value: string,
  max: number,
): FieldProblem | null {
  if (value.trim() === "") return problem(field, "blank");
  if ([...value].length > max) return problem(field, "too_long", max);
  if (CONTROL_CHARACTER.test(value)) return problem(field, "control_character");
  if (field === "name" && FORMAT_CHARACTER.test(value)) {
    return problem(field, "format_character");
  }
  return null;
}

function isRealDate(date: string): boolean {
  if (!DATE_PATTERN.test(date)) return false;
  const [year, month, day] = date.split("-").map(Number);
  const moment = new Date(Date.UTC(year, month - 1, day));
  return (
    moment.getUTCFullYear() === year &&
    moment.getUTCMonth() === month - 1 &&
    moment.getUTCDate() === day
  );
}

function repeatProblem(draft: ScheduleDraft): FieldProblem | null {
  const needsTime = draft.kind !== "every_hours";
  if (draft.kind === "once" && !isRealDate(draft.date)) {
    return problem("repeat", "bad_date");
  }
  if (needsTime && !TIME_PATTERN.test(draft.time)) {
    return problem("repeat", "bad_time");
  }
  if (draft.kind === "weekly" && draft.days.length === 0) {
    return problem("repeat", "no_day");
  }
  if (draft.kind === "every_hours") {
    const hours = draft.hours.trim();
    const isInRange =
      WHOLE_NUMBER.test(hours) &&
      Number(hours) >= MIN_HOURS &&
      Number(hours) <= MAX_HOURS;
    if (!isInRange) return problem("repeat", "hours_out_of_range");
  }
  return null;
}

/**
 * The quick checks before a save. They mirror the server's rules for each
 * field; the server decides, and its answer replaces these (0025 D19). A
 * once time in the past and a name that another schedule has are the
 * server's to judge.
 */
export function draftProblems(draft: ScheduleDraft): FieldProblem[] {
  const found = [
    textProblem("name", draft.name, NAME_MAX_CHARACTERS),
    textProblem("text", draft.text, TEXT_MAX_CHARACTERS),
    repeatProblem(draft),
    draft.targetAgentId === "" ? problem("target", "target_missing") : null,
  ];
  return found.filter((item): item is FieldProblem => item !== null);
}

/** The failing fields of a refused save, from the server's `fields`. */
export function fieldProblemsOf(
  fields: FieldFailure[] | undefined,
): FieldProblem[] {
  return fields ?? [];
}

/** One failing field as a sentence: "Name must not be blank". */
export function problemText(item: FieldProblem): string {
  const label = FIELD_LABELS[item.field as FieldName] ?? item.field;
  return `${label} ${item.message}`;
}

/** The problems of one field, in the order given. */
export function problemsOfField(
  problems: readonly FieldProblem[],
  field: FieldName,
): FieldProblem[] {
  return problems.filter((item) => item.field === field);
}
