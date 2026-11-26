// SPDX-License-Identifier: AGPL-3.0-or-later

/** The setting that says whether claude sessions report their status
 * through hooks (0024 FR-024); "on" by default. */
export const HOOK_STATUS_KEY = "hook_status";
export const SWITCH_ON = "on";
export const SWITCH_OFF = "off";

export const HOOK_STATUS_HELP_TEXT =
  "Claude agents report their turns, permission requests and compaction to the office, so the status is exact. Off: the office guesses the status from the quiet period. The change applies from the next session start of each agent.";

export const SWITCH_ON_TEXT = "On: claude sessions report their status.";
export const SWITCH_OFF_TEXT = "Off: the status follows the quiet period.";

/** A stored value that is not "off" reads as on, the default. */
export function isSwitchOn(value: string): boolean {
  return value !== SWITCH_OFF;
}

export function switchValueOf(isOn: boolean): string {
  return isOn ? SWITCH_ON : SWITCH_OFF;
}
