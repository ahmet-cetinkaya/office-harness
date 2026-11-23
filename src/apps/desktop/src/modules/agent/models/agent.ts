// SPDX-License-Identifier: AGPL-3.0-or-later

/** The status words the server sends (FR-014). */
const STATUS_TEXT: Record<string, string> = {
  starting: "starting",
  working: "working",
  waiting_for_input: "waiting for input",
  exited: "exited",
  failed: "failed",
  stopped: "stopped",
  interrupted: "interrupted",
  asleep: "asleep",
};

const ACTIVE_STATUSES: readonly string[] = [
  "starting",
  "working",
  "waiting_for_input",
];

/** The last task the office gave an agent, or a user set (0011). */
export interface AgentTask {
  text: string;
  /** Unix ms, from the server clock. */
  set_at: number;
  source: "directive" | "user";
}

/** The own worktree of an agent (0010, FR-003). */
export interface AgentWorktree {
  repository: string;
  base: string;
  branch: string;
  folder: string;
}

/** What marks a temporary agent: its one job and who created it (0020). */
export interface AgentTemp {
  objective: string;
  /** "user", or the id of the agent that created it. */
  creator: string;
}

export interface Agent {
  id: string;
  name: string;
  command: string;
  working_folder: string | null;
  project_id: string | null;
  machine_id: string;
  status: string;
  created_at: number;
  /** The agent's last output line, plain text, or null before any output. */
  last_line: string | null;
  /** Absent or null when the agent has no task text. */
  task?: AgentTask | null;
  /** Absent or null when the agent works in its own folder. */
  worktree?: AgentWorktree | null;
  /** Null, or absent, for an agent that is not a temp. */
  temp?: AgentTemp | null;
}

export interface Project {
  id: string;
  name: string;
  folder: string;
}

/** A null value is "no value": the CLI does not get the variable (0009/FR-011). */
export interface EnvironmentEntry {
  name: string;
  value: string | null;
}

/** What office-harness adds when it starts one agent (FR-022). */
export interface Additions {
  flags: string[];
  env: EnvironmentEntry[];
  startup_prompt: string | null;
}

export interface AdditionsView extends Additions {
  updated_at: number;
  applies_next_session: boolean;
}

export interface Session {
  id: string;
  agent_id: string;
  cli_session_id: string | null;
  continued_from: string | null;
  additions_snapshot: Additions;
  started_at: number;
  ended_at: number | null;
  exit_result: string | null;
  can_continue: boolean;
  reason?: string;
}

export interface NewAgent {
  name: string;
  command: string;
  working_folder?: string;
  project_id?: string;
  /** The own worktree choice; `base` and `branch_name` need it (0010). */
  own_worktree?: boolean;
  base?: string;
  branch_name?: string;
  /** A temporary agent needs an objective (0020). */
  temp?: boolean;
  objective?: string;
}

export const OBJECTIVE_MAX_CHARACTERS = 8192;
export const OBJECTIVE_HINT = `The one job of the temp. It must not be blank and has at most ${OBJECTIVE_MAX_CHARACTERS.toLocaleString("en-US")} characters, on one line.`;
export const OBJECTIVE_REQUIRED_TEXT = "A temp needs an objective";
export const OBJECTIVE_ONE_LINE_TEXT = "The objective must be one line.";
const LINE_BREAK = /[\r\n]/;
export const OBJECTIVE_TOO_LONG_TEXT = `The objective has at most ${OBJECTIVE_MAX_CHARACTERS.toLocaleString("en-US")} characters`;

/** The refusal of a temp objective, or null when it can be sent. */
export function objectiveProblem(objective: string): string | null {
  const sent = objective.trim();
  if (sent === "") return OBJECTIVE_REQUIRED_TEXT;
  if (LINE_BREAK.test(sent)) return OBJECTIVE_ONE_LINE_TEXT;
  if (objective.length > OBJECTIVE_MAX_CHARACTERS) {
    return OBJECTIVE_TOO_LONG_TEXT;
  }
  return null;
}

const SCHEDULE_CREATOR_PREFIX = "schedule:";
const SCHEDULE_ID_SHORT_LENGTH = 8;

/** The creator of a temp as text: the person, the agent's name, or its id. */
export function creatorText(
  creator: string,
  agents: readonly Pick<Agent, "id" | "name">[],
): string {
  if (creator === "user") return "the person";
  if (creator === "office") return "the office";
  if (creator.startsWith(SCHEDULE_CREATOR_PREFIX)) {
    const id = creator.slice(SCHEDULE_CREATOR_PREFIX.length);
    return `schedule ${id.slice(0, SCHEDULE_ID_SHORT_LENGTH)}`;
  }
  return agents.find((agent) => agent.id === creator)?.name ?? creator;
}

export interface NewProject {
  name: string;
  folder: string;
}

/** A status as text. A word that this app does not know is shown as sent. */
export function statusText(status: string): string {
  return STATUS_TEXT[status] ?? status.replaceAll("_", " ");
}

/** True while a session runs: starting, working, or waiting for input. */
export function isActive(status: string): boolean {
  return ACTIVE_STATUSES.includes(status);
}

/** What the app shows for an agent while its stop request is open. */
export const STOPPING_TEXT = "Stopping";

/** The status text to show: "Stopping" while a stop request is open. */
export function shownStatusText(agent: Agent, isStopping: boolean): string {
  return isStopping ? STOPPING_TEXT : statusText(agent.status);
}
