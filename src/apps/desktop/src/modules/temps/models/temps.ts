// SPDX-License-Identifier: AGPL-3.0-or-later

import type { Agent } from "../../agent";

/** One running or waiting temp, as `temp.list` answers it (0020). */
export interface TempRow {
  agent: Agent;
  /** "the person", the creator's name, or null when that agent is gone. */
  creator_name: string | null;
  parent_name: string | null;
  run_seconds: number;
  /** Null for a temp that is not running or asleep. */
  quiet_seconds: number | null;
}

export interface TempReport {
  kind: string;
  text: string;
}

/** One temp that ended, as `temp.list` answers it. */
export interface FinishedTemp {
  id: string;
  agent_id: string;
  name: string;
  objective: string;
  creator: string;
  creator_name: string;
  parent_name: string | null;
  started_at: number;
  ended_at: number;
  reason: string;
  report: TempReport | null;
  kept_folder: string | null;
  kept_branch: string | null;
  removal_failure: string | null;
}

export interface TempList {
  temps: TempRow[];
  finished: FinishedTemp[];
}

export const EMPTY_TEMP_LIST: TempList = { temps: [], finished: [] };

export const NO_TEMPS_TEXT = "No temps.";
export const NO_FINISHED_TEMPS_TEXT = "No finished temps.";
export const NOTHING_KEPT_TEXT =
  "nothing kept; files that git ignores were removed with the worktree";
export const NOT_RUNNING_TEXT = "not running";
export const GONE_CREATOR_TEXT = "an agent that is gone";

const SECONDS_PER_MINUTE = 60;
const MINUTES_PER_HOUR = 60;

/** A span as short text: "45 s", "12 min", "2 h", or "1 h 5 min". */
export function durationText(totalSeconds: number): string {
  const seconds = Math.max(0, Math.floor(totalSeconds));
  if (seconds < SECONDS_PER_MINUTE) return `${seconds} s`;
  const totalMinutes = Math.floor(seconds / SECONDS_PER_MINUTE);
  if (totalMinutes < MINUTES_PER_HOUR) return `${totalMinutes} min`;
  const hours = Math.floor(totalMinutes / MINUTES_PER_HOUR);
  const minutes = totalMinutes % MINUTES_PER_HOUR;
  return minutes === 0 ? `${hours} h` : `${hours} h ${minutes} min`;
}

/** The quiet time as text; a temp that does not run has none. */
export function quietText(quietSeconds: number | null): string {
  return quietSeconds === null ? NOT_RUNNING_TEXT : durationText(quietSeconds);
}

/** The end reason as a sentence start: the server's words, capitalised. */
export function reasonText(reason: string): string {
  return reason === "" ? reason : reason[0].toUpperCase() + reason.slice(1);
}

/** The creator of a temp row; a creator that is gone is said so. */
export function creatorNameText(name: string | null): string {
  return name ?? GONE_CREATOR_TEXT;
}

/** The report of a finished temp, or null when it sent none. */
export function reportLine(entry: FinishedTemp): string | null {
  if (entry.report === null) return null;
  return `${reasonText(entry.report.kind)} report: ${entry.report.text}`;
}

/** What stayed after the end: the folder and branch, or the nothing-kept text. */
export function keptText(entry: FinishedTemp): string {
  const parts: string[] = [];
  if (entry.kept_folder !== null) parts.push(`folder ${entry.kept_folder}`);
  if (entry.kept_branch !== null) parts.push(`branch ${entry.kept_branch}`);
  const kept =
    parts.length === 0 ? NOTHING_KEPT_TEXT : `Kept ${parts.join(", ")}`;
  return entry.removal_failure === null
    ? kept
    : `${kept}. The worktree was not removed: ${entry.removal_failure}`;
}

/** The ended time as local date and time text. */
export function endedText(endedAt: number): string {
  return new Date(endedAt).toLocaleString();
}

/** The accessible name of a temp row: name, status, and creator. */
export function rowName(row: TempRow, statusLabel: string): string {
  return `${row.agent.name}, ${statusLabel}, created by ${creatorNameText(row.creator_name)}`;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

/** The `temp.list` answer; a malformed answer reads as an empty list. */
export function readTempList(result: unknown): TempList {
  if (!isRecord(result)) return EMPTY_TEMP_LIST;
  return {
    temps: Array.isArray(result.temps) ? (result.temps as TempRow[]) : [],
    finished: Array.isArray(result.finished)
      ? (result.finished as FinishedTemp[])
      : [],
  };
}
