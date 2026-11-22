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
  isAfterUncleanStop: boolean;
}

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
    isAfterUncleanStop: fields.after_unclean_stop === true,
  };
}

/** The entries of a `resume.plan` answer, in the order sent; bad input is no entry. */
export function readPlan(value: unknown): PlanEntry[] {
  const agents = (value as { agents?: unknown } | null)?.agents;
  if (!Array.isArray(agents)) return [];
  return agents.flatMap((agent) => readPlanEntry(agent) ?? []);
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
  if (entry.isAfterUncleanStop) marks.push(AFTER_UNCLEAN_STOP_TEXT);
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

/** Why Confirm is unavailable, or null when it may be pressed (FR-002, FR-008). */
export type ConfirmBlock = "nothing_checked" | "needs_acknowledgement";

export function confirmBlock(
  entries: PlanEntry[],
  checkedIds: string[],
  isAcknowledged: boolean,
): ConfirmBlock | null {
  if (checkedIds.length === 0) return "nothing_checked";
  const isMarkedChecked = entries.some(
    (entry) => entry.isAfterUncleanStop && checkedIds.includes(entry.agent_id),
  );
  return isMarkedChecked && !isAcknowledged ? "needs_acknowledgement" : null;
}

/** True when any listed agent is marked, so the warning shows (FR-008). */
export function hasUncleanStopMark(entries: PlanEntry[]): boolean {
  return entries.some((entry) => entry.isAfterUncleanStop);
}
