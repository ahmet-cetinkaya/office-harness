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

/** The four limits, in the order the dialog lists them. */
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
];

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
