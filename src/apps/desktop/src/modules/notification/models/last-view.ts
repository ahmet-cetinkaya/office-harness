// SPDX-License-Identifier: AGPL-3.0-or-later

/** The last view the app restores from (FR-008, D6). */
export interface LastView {
  /** The chosen project id, or null for "All projects". */
  project: string | null;
  /** The selected agent id, or none. */
  agent: string | null;
  /** Whether that agent's terminal was open. */
  terminal_open: boolean;
}

/** Parses the stored text. Anything that is not the documented shape is
 * ignored, so the app falls back to its default view (D6). */
export function parseLastView(text: unknown): LastView | null {
  if (typeof text !== "string" || text === "") return null;
  let view: unknown;
  try {
    view = JSON.parse(text);
  } catch {
    return null;
  }
  if (typeof view !== "object" || view === null) return null;
  const fields = view as Record<string, unknown>;
  const project = fields.project;
  const agent = fields.agent;
  if (
    !(project === null || typeof project === "string") ||
    !(agent === null || typeof agent === "string") ||
    typeof fields.terminal_open !== "boolean"
  ) {
    return null;
  }
  return { project, agent, terminal_open: fields.terminal_open };
}

/** How the restore treats the stored agent (US3 scenario 2, edge cases). */
export type AgentOutcome =
  { kind: "present"; agentId: string } | { kind: "missing" } | { kind: "none" };

/** The restore's plan for one stored view, against the office as it is now. */
export function resolveLastView(
  view: LastView,
  projectIds: readonly string[],
  agentIds: readonly string[],
): { choice: string | null; agent: AgentOutcome } {
  const projectExists =
    view.project !== null && projectIds.includes(view.project);
  const choice = projectExists ? view.project : null;
  if (view.agent === null) return { choice, agent: { kind: "none" } };
  return {
    choice,
    agent: agentIds.includes(view.agent)
      ? { kind: "present", agentId: view.agent }
      : { kind: "missing" },
  };
}
