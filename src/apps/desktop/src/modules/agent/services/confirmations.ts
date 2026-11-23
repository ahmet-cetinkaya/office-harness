// SPDX-License-Identifier: AGPL-3.0-or-later

/** What the user must confirm before it is sent. */
export type Confirmation =
  | { kind: "stop" }
  | { kind: "delete" }
  | { kind: "end_temp" }
  | { kind: "new_task"; isRunning: boolean }
  | { kind: "continue"; sessionId: string; isRunning: boolean };

export interface ConfirmationText {
  title: string;
  body: string;
  confirmLabel: string;
  isDestructive: boolean;
}

const ENDS_RUNNING_SESSION =
  "The running session ends first and its terminal closes.";

/** What ending a temp, or stopping it alone, removes and what stays (0020). */
export function tempRemovalText(agentName: string): string {
  return `${agentName} will be removed. Work that is not merged stays in its worktree and branch. Files that git ignores are removed with a removed worktree.`;
}

export function confirmationText(
  confirmation: Confirmation,
  agentName: string,
): ConfirmationText {
  switch (confirmation.kind) {
    case "stop":
      return {
        title: `Stop ${agentName}?`,
        body: "The agent's CLI is ended and the session stops.",
        confirmLabel: "Stop agent",
        isDestructive: true,
      };
    case "end_temp":
      return {
        title: `End ${agentName}?`,
        body: tempRemovalText(agentName),
        confirmLabel: "End temp",
        isDestructive: true,
      };
    case "delete":
      return {
        title: `Delete ${agentName}?`,
        body: "This removes the agent, its office additions, and its session list. A running session ends. The action record keeps its entries.",
        confirmLabel: "Delete agent",
        isDestructive: true,
      };
    case "new_task":
      return {
        title: `Start a new task on ${agentName}?`,
        body: confirmation.isRunning
          ? `${ENDS_RUNNING_SESSION} A new CLI session then starts.`
          : "A new CLI session starts.",
        confirmLabel: "Start new task",
        isDestructive: confirmation.isRunning,
      };
    case "continue":
      return {
        title: `Continue this session of ${agentName}?`,
        body: confirmation.isRunning
          ? `${ENDS_RUNNING_SESSION} The CLI then continues the session you picked.`
          : "The CLI starts so that it continues the session you picked.",
        confirmLabel: "Continue session",
        isDestructive: confirmation.isRunning,
      };
  }
}

/** A new task or a continue asks only when it would end a running session. */
export function needsConfirmation(confirmation: Confirmation): boolean {
  if (confirmation.kind === "new_task" || confirmation.kind === "continue") {
    return confirmation.isRunning;
  }
  return true;
}
