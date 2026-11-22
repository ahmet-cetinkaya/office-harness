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
