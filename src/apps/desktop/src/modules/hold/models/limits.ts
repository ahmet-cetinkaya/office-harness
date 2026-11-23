// SPDX-License-Identifier: AGPL-3.0-or-later

/** One rate limit of the office settings (contract: settings keys, FR-009). */
export interface LimitSetting {
  key: string;
  label: string;
  defaultValue: number;
}

export const LIMIT_MIN = 1;
export const LIMIT_MAX = 1000;
export const LIMIT_INVALID_MESSAGE =
  "a limit is a whole number of at least 1 and at most 1000";

/** The five limits, in the order the dialog lists them. */
export const LIMIT_SETTINGS: readonly LimitSetting[] = [
  {
    key: "limit_directives_per_agent",
    label: "Directives sent by one agent",
    defaultValue: 10,
  },
  {
    key: "limit_directives_per_target",
    label: "Directives sent to one target agent",
    defaultValue: 3,
  },
  {
    key: "limit_creates_per_agent",
    label: "Agents created by one agent",
    defaultValue: 5,
  },
  {
    key: "limit_lifecycle_per_agent",
    label: "Starts, stops, and new tasks by one agent",
    defaultValue: 10,
  },
  {
    key: "limit_reports_per_agent",
    label: "Reports sent by one agent",
    defaultValue: 5,
  },
];

/** A temporary agent setting: a whole number in its own range (0020, D2). */
export interface TempSetting {
  key: string;
  label: string;
  defaultValue: number;
  min: number;
  max: number;
  invalidMessage: string;
}

/** The two settings of temporary agents, in the order the dialog lists them. */
export const TEMP_SETTINGS: readonly TempSetting[] = [
  {
    key: "temp_cap",
    label: "Temp cap",
    defaultValue: 8,
    min: 1,
    max: 100,
    invalidMessage: "temp_cap is a whole number from 1 to 100",
  },
  {
    key: "temp_idle_minutes",
    label: "Temp idle limit, minutes",
    defaultValue: 30,
    min: 5,
    max: 1440,
    invalidMessage:
      "temp_idle_minutes is a whole number of minutes from 5 to 1440",
  },
];

/** The schedule cap: how many schedules the office keeps (0025). */
export const SCHEDULE_SETTINGS: readonly TempSetting[] = [
  {
    key: "schedule_cap",
    label: "Schedule cap",
    defaultValue: 50,
    min: 1,
    max: 200,
    invalidMessage: "schedule_cap is a whole number from 1 to 200",
  },
];

/** Every setting that the limits dialog stores, rate limits first. */
export const ALL_LIMIT_SETTINGS: readonly {
  key: string;
  defaultValue: number;
}[] = [...LIMIT_SETTINGS, ...TEMP_SETTINGS, ...SCHEDULE_SETTINGS];

const WHOLE_NUMBER = /^\d+$/;

/** The message that refuses the text as a limit, or null when it is one. */
export function limitProblem(text: string): string | null {
  const trimmed = text.trim();
  if (!WHOLE_NUMBER.test(trimmed)) return LIMIT_INVALID_MESSAGE;
  const value = Number(trimmed);
  return value >= LIMIT_MIN && value <= LIMIT_MAX
    ? null
    : LIMIT_INVALID_MESSAGE;
}

/** The message that refuses the text as the temp setting, or null. */
export function tempProblem(text: string, setting: TempSetting): string | null {
  const trimmed = text.trim();
  if (!WHOLE_NUMBER.test(trimmed)) return setting.invalidMessage;
  const value = Number(trimmed);
  return value >= setting.min && value <= setting.max
    ? null
    : setting.invalidMessage;
}

/** The first setting whose text is refused, with the reason, or null. */
export function firstLimitProblem(
  values: Record<string, string>,
): { limit: { key: string; label: string }; problem: string } | null {
  for (const limit of LIMIT_SETTINGS) {
    const problem = limitProblem(values[limit.key]);
    if (problem !== null) return { limit, problem };
  }
  for (const setting of [...TEMP_SETTINGS, ...SCHEDULE_SETTINGS]) {
    const problem = tempProblem(values[setting.key], setting);
    if (problem !== null) return { limit: setting, problem };
  }
  return null;
}
