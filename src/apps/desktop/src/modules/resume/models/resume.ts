// SPDX-License-Identifier: AGPL-3.0-or-later

import { formatTimestamp } from "$shared/utils";

/** The agent status the resume action counts (contract: resume.plan). */
const INTERRUPTED_STATUS = "interrupted";

/** One agent of a `resume.plan` answer, in start order. */
export interface PlanEntry {
  agent_id: string;
  /** Unix time in milliseconds when the last session started; null without one. */
  lastSessionStartedAt: number | null;
  canContinue: boolean;
  /** Why the last session cannot be continued. */
  reason: string | null;
  isHeld: boolean;
  uncleanStop: UncleanStopFinding;
}

/**
 * What the orphan sweep found for the agent's lost session (0017, D8):
 * nothing to say, leftovers it ended, or a process that may still run.
 */
export type UncleanStopFinding =
  | { kind: "none" }
  | { kind: "ended"; count: number }
  | { kind: "may_run"; reason: string | null };

const NO_FINDING: UncleanStopFinding = { kind: "none" };

/** How the dialog names an agent: its name and its role as text. */
export interface AgentIdentity {
  name: string;
  role: string;
}

export type ResultKind =
  "checking" | "continued" | "new_session" | "held" | "skipped";

/** One `resume.result` event. */
export interface ResumeResult {
  agent_id: string;
  result: ResultKind;
  continued: boolean;
  reason: string | null;
  isFinal: boolean;
}

export const NOTHING_CHECKED_TEXT = "Check at least one agent";
export const ACKNOWLEDGEMENT_NEEDED_TEXT =
  "Tick the acknowledgement to confirm";
export const ACKNOWLEDGEMENT_LABEL =
  "I understand that a second copy may start";
export const UNCLEAN_STOP_WARNING =
  "The server stopped uncleanly. CLI processes of the marked agents from before may still run, and resuming can start a second copy next to them.";
export const ON_HOLD_TEXT = "On hold";
export const AFTER_UNCLEAN_STOP_TEXT =
  "Was running when the server stopped uncleanly";
export const WAITING_FOR_RESULT_TEXT = "Waiting to start";

/** The note of an entry whose leftovers the sweep ended (0017, D9). */
export function endedLeftoversText(count: number): string {
  const noun = count === 1 ? "process" : "processes";
  return `Ended ${count} ${noun} left from the last run`;
}

const RESULT_KINDS: readonly string[] = [
  "checking",
  "continued",
  "new_session",
  "held",
  "skipped",
];

/** How many agents of the office are interrupted (FR-001). */
export function interruptedCount(agents: { status: string }[]): number {
  return agents.filter((agent) => agent.status === INTERRUPTED_STATUS).length;
}

function readText(value: unknown): string | null {
  return typeof value === "string" ? value : null;
}

function readFinding(fields: Record<string, unknown>): UncleanStopFinding {
  const finding = fields.unclean_stop;
  if (typeof finding === "object" && finding !== null) {
    const { ended, may_run: mayRun } = finding as Record<string, unknown>;
    if (typeof mayRun === "string") return { kind: "may_run", reason: mayRun };
    if (typeof ended === "number" && ended > 0) {
      return { kind: "ended", count: ended };
    }
    return NO_FINDING;
  }
  // An older server sends only the boolean, which is true for may_run.
  return fields.unclean_stop === undefined && fields.after_unclean_stop === true
    ? { kind: "may_run", reason: null }
    : NO_FINDING;
}

function readPlanEntry(value: unknown): PlanEntry | null {
  if (typeof value !== "object" || value === null) return null;
  const fields = value as Record<string, unknown>;
  if (typeof fields.agent_id !== "string") return null;
  const session = fields.last_session as { started_at?: unknown } | null;
  return {
    agent_id: fields.agent_id,
    lastSessionStartedAt:
      typeof session?.started_at === "number" ? session.started_at : null,
    canContinue: fields.can_continue === true,
    reason: readText(fields.reason),
    isHeld: fields.held === true,
    uncleanStop: readFinding(fields),
  };
}

/** The entries of a `resume.plan` answer, in the order sent; bad input is no entry. */
export function readPlan(value: unknown): PlanEntry[] {
  const agents = (value as { agents?: unknown } | null)?.agents;
  if (!Array.isArray(agents)) return [];
  return agents.flatMap((agent) => readPlanEntry(agent) ?? []);
}

/** True while the server's sweep runs and the plan is not ready (0017, FR-007). */
export function isSweepingPlan(value: unknown): boolean {
  return (value as { sweeping?: unknown } | null)?.sweeping === true;
}

/** The `resume.result` event as a result, or null when it is not one. */
export function readResult(event: unknown): ResumeResult | null {
  if (typeof event !== "object" || event === null) return null;
  const fields = event as Record<string, unknown>;
  const { agent_id: agentId, result } = fields;
  if (typeof agentId !== "string" || typeof result !== "string") return null;
  if (!RESULT_KINDS.includes(result)) return null;
  return {
    agent_id: agentId,
    result: result as ResultKind,
    continued: fields.continued === true,
    reason: readText(fields.reason),
    isFinal: fields.is_final === true,
  };
}

/** What will happen to the agent, as the list shows it before Confirm. */
export function planText(entry: PlanEntry): string {
  if (entry.canContinue && entry.lastSessionStartedAt !== null) {
    return `Continues the session started ${formatTimestamp(entry.lastSessionStartedAt)}`;
  }
  return entry.reason === null
    ? "Starts a new session"
    : `Starts a new session: ${entry.reason}`;
}

/** The marks of an entry, as words. */
export function marksOf(entry: PlanEntry): string[] {
  const marks: string[] = [];
  if (entry.isHeld) marks.push(ON_HOLD_TEXT);
  const { uncleanStop } = entry;
  if (uncleanStop.kind === "ended") {
    marks.push(endedLeftoversText(uncleanStop.count));
  }
  if (uncleanStop.kind === "may_run") {
    marks.push(
      uncleanStop.reason === null
        ? AFTER_UNCLEAN_STOP_TEXT
        : `${AFTER_UNCLEAN_STOP_TEXT}: ${uncleanStop.reason}`,
    );
  }
  return marks;
}

function withReason(lead: string, reason: string | null): string {
  return reason === null ? lead : `${lead}: ${reason}`;
}

/** One result as text; never colour alone (FR-006). */
export function resultText(result: ResumeResult): string {
  switch (result.result) {
    case "checking":
      return result.continued
        ? "Checking: continuing a session"
        : "Checking: starting a new session";
    case "continued":
      return "Continued";
    case "new_session":
      return withReason("New session", result.reason);
    case "held":
      return withReason("On hold", result.reason);
    case "skipped":
      return withReason("Skipped", result.reason);
  }
}

/** True when the sweep could not rule out a second copy (0013/FR-008, 0017). */
function isMayRun(entry: PlanEntry): boolean {
  return entry.uncleanStop.kind === "may_run";
}

/** Why Confirm is unavailable, or null when it may be pressed (FR-002, FR-008). */
export type ConfirmBlock = "nothing_checked" | "needs_acknowledgement";

export function confirmBlock(
  entries: PlanEntry[],
  checkedIds: string[],
  isAcknowledged: boolean,
): ConfirmBlock | null {
  if (checkedIds.length === 0) return "nothing_checked";
  const isMarkedChecked = entries.some(
    (entry) => isMayRun(entry) && checkedIds.includes(entry.agent_id),
  );
  return isMarkedChecked && !isAcknowledged ? "needs_acknowledgement" : null;
}

/** True when any listed agent may still run, so the warning shows (FR-008). */
export function hasUncleanStopMark(entries: PlanEntry[]): boolean {
  return entries.some(isMayRun);
}
