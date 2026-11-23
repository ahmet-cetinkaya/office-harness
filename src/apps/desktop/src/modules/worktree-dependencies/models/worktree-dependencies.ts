// SPDX-License-Identifier: AGPL-3.0-or-later

/** The setting that says whether new own worktrees share the installed
 * packages (0021 FR-001); "on" by default. */
export const WORKTREE_DEPENDENCIES_KEY = "worktree_dependency_links";
export const SWITCH_ON = "on";
export const SWITCH_OFF = "off";

export const WORKTREE_DEPENDENCIES_HELP_TEXT =
  "New own worktrees share the installed Node.js packages (node_modules) of the checkout, so an agent can build and test without an install. An install, update, or removal of packages in such a worktree changes the checkout's packages too. On Windows, no packages are shared.";

export const SWITCH_ON_TEXT = "On: new own worktrees share the packages.";
export const SWITCH_OFF_TEXT =
  "Off: each agent installs the packages in its new worktree.";

/** A stored value that is not "off" reads as on, the default. */
export function isSwitchOn(value: string): boolean {
  return value !== SWITCH_OFF;
}

export function switchValueOf(isOn: boolean): string {
  return isOn ? SWITCH_ON : SWITCH_OFF;
}
