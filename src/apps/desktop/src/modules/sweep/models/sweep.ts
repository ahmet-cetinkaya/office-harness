// SPDX-License-Identifier: AGPL-3.0-or-later

/** What the panel and the resume dialog say while the sweep runs (0017, FR-007). */
export const SWEEP_RUNNING_TEXT = "Ending processes left from the last run";
/** Spoken once when a sweep that was seen running ends; never shown. */
export const SWEEP_FINISHED_TEXT =
  "The sweep of processes left from the last run is finished";

export type SweepState = "running" | "done";

/** The state of a `sweep.state` answer or an `office.sweep` event; null when it is neither. */
export function readSweepState(value: unknown): SweepState | null {
  const state = (value as { state?: unknown } | null)?.state;
  return state === "running" || state === "done" ? state : null;
}

/** The text of the panel's status region for what the controller saw. */
export function sweepStatusText(
  phase: "idle" | "running" | "finished",
): string {
  if (phase === "running") return SWEEP_RUNNING_TEXT;
  return phase === "finished" ? SWEEP_FINISHED_TEXT : "";
}
