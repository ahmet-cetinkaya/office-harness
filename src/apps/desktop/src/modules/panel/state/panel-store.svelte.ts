// SPDX-License-Identifier: AGPL-3.0-or-later

import type { CoreClient } from "../../../core/api";
import {
  createNotificationController,
  createOsNotificationService,
  readLastView,
  resolveLastView,
} from "../../notification";
import {
  createOfficeStore,
  LAST_AGENT_GONE_LEAD,
  LAST_AGENT_GONE_MESSAGE,
} from "../../agent";
import { createHoldController } from "../../hold";
import { createNeedsYouController } from "../../needs-you";
import { createResumeController } from "../../resume";
import { createSweepController } from "../../sweep";
import { createSleepController } from "../../sleep";
import { createTempsController } from "../../temps";
import { createTemplateController } from "../../template";
import { createHookStatusController } from "../../hook-status";
import { createWorktreeDependenciesController } from "../../worktree-dependencies";

export type OpenView = "panel" | "scene";

/**
 * The panel's stores over one client: the office, the project choice, the
 * notification list with its OS notifications, the holds (0007), Needs you
 * (0008), the resume of interrupted agents (0013), the orphan sweep (0017), the templates (0009), and the last view (0003).
 */
export function createPanelStore(client: CoreClient) {
  const office = createOfficeStore(client, () => void notifications.load());
  const needsYou = createNeedsYouController(client);
  const notifications = createNotificationController(
    client,
    createOsNotificationService(),
    () =>
      openView === "panel" ? office.snapshot.selectedAgentId : sceneOpenAgentId,
    () => document.hasFocus(),
    needsYou.hideOrigin,
    needsYou.showOrigin,
  );
  const templates = createTemplateController(client);
  const hold = createHoldController(
    client,
    (agentId) =>
      office.snapshot.agents.find((agent) => agent.id === agentId)?.name ??
      null,
  );
  const resume = createResumeController(client);
  const sweep = createSweepController(client);
  const sleep = createSleepController(client);
  const temps = createTempsController(client);
  const worktreeDependencies = createWorktreeDependenciesController(client);
  const hookStatus = createHookStatusController(client);
  // The restore is the one reader of `last_view` and the only place the
  // stored view word is applied. The shell writes the last view only after
  // the restore has settled, or an eager write would clobber the stored
  // value. `isLastViewRestoring` is set before the read starts, so a
  // reconnect or a second trigger during the read cannot start a second
  // restore.
  let isLastViewRestoring = $state(false);
  let isLastViewRestored = $state(false);
  const lastView = {
    get isRestored(): boolean {
      return isLastViewRestored;
    },
  };
  // The shell's open view. A switch the user makes before the restore has
  // settled wins: the stored view is the last session's place, not today's
  // instruction.
  let openView = $state<OpenView>("panel");
  let hasUserSwitched = false;
  // The restored agent whose read the scene's restore skipped. The panel's
  // next show reads it once; a user selection reads it already.
  let unreadRestoredAgentId: string | null = null;
  // The agent whose terminal the scene embeds, or null. The scene's desk
  // terminal is on screen like the panel's selected one (FR-006, FR-009).
  let sceneOpenAgentId: string | null = null;
  let projectChoice = $state<string | null>(null);

  // Opening an agent's terminal reads its notifications (FR-009). The select
  // sits here, so every route through the panel reads, not just the rows.
  const controller = {
    ...office.controller,
    select(agentId: string): void {
      unreadRestoredAgentId = null;
      office.controller.select(agentId);
      void notifications.readAgent(agentId);
    },
  };

  // The restore selects without reading: FR-009 marks notifications read when
  // the agent's terminal opens, and only the panel shows it. The scene's
  // restore leaves them unread until the panel shows the terminal.
  function selectRestoredAgent(agentId: string): void {
    office.controller.select(agentId);
    if (openView === "panel") {
      void notifications.readAgent(agentId);
    } else {
      unreadRestoredAgentId = agentId;
    }
  }

  function readRestoredAgentOnPanel(): void {
    const pendingAgentId = unreadRestoredAgentId;
    unreadRestoredAgentId = null;
    // A selection that changed under the pending read (the agent was
    // deleted) shows no terminal of that agent, so its rows stay unread.
    if (pendingAgentId === null) return;
    if (pendingAgentId !== office.snapshot.selectedAgentId) return;
    void notifications.readAgent(pendingAgentId);
  }

  /**
   * The last view's restore (FR-008, D6), run once from the shell when the
   * office's first load has brought its projects and agents, whichever view
   * is open. A view this app cannot apply falls back piece by piece, and
   * each fallback says what it fell back from (US3).
   */
  async function restoreLastView(): Promise<void> {
    if (isLastViewRestoring || isLastViewRestored) return;
    isLastViewRestoring = true;
    const stored = await readLastView(client);
    if (stored !== null) {
      const plan = resolveLastView(
        stored,
        office.snapshot.projects.map((project) => project.id),
        office.snapshot.agents.map((agent) => agent.id),
      );
      projectChoice = plan.choice;
      if (!hasUserSwitched) openView = stored.view;
      if (plan.agent.kind === "missing") {
        controller.showNotice(LAST_AGENT_GONE_LEAD, LAST_AGENT_GONE_MESSAGE);
      } else if (plan.agent.kind === "present") {
        selectRestoredAgent(plan.agent.agentId);
      }
    }
    // The shell writes the last view only from here on, so its eager write
    // cannot land between this read and its use.
    isLastViewRestored = true;
  }

  return {
    /** The project id, or null for "All projects". */
    get projectChoice(): string | null {
      return projectChoice;
    },
    setProjectChoice(choice: string | null): void {
      projectChoice = choice;
    },
    /** The open view: the panel, or the office scene (0005 D6). */
    get openView(): OpenView {
      return openView;
    },
    /** The user's own switch of the view. */
    switchView(view: OpenView): void {
      hasUserSwitched = true;
      openView = view;
      if (view === "panel") readRestoredAgentOnPanel();
    },
    /**
     * The shell reports the scene's open desk, or null when the desk closes
     * or the scene is left. Opening a desk reads that agent's rows.
     */
    setSceneOpenAgent(agentId: string | null): void {
      sceneOpenAgentId = agentId;
      if (agentId !== null) void notifications.readAgent(agentId);
    },
    restoreLastView,
    office: {
      get snapshot() {
        return office.snapshot;
      },
      controller,
      client,
    },
    notifications,
    needsYou,
    hold,
    resume,
    sweep,
    sleep,
    temps,
    worktreeDependencies,
    hookStatus,
    templates,
    lastView,
    client,
  };
}

export type PanelStore = ReturnType<typeof createPanelStore>;
