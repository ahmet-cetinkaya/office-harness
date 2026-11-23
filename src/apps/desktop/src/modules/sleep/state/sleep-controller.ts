// SPDX-License-Identifier: AGPL-3.0-or-later

import {
  toCoreRequestError,
  type CoreClient,
  type ServerEvent,
} from "../../../core/api";
import {
  ASLEEP_STATUS,
  formProblem,
  isWakeBusy,
  NO_LONGER_ASLEEP_TEXT,
  readIdleSleepForm,
  readWakeResult,
  settingValueOf,
  type IdleSleepForm,
  type WakeEntry,
} from "../models/sleep";
import { createSleepService } from "../services/sleep-service";

export interface SleepSnapshot {
  /** The newest wake entry of each agent, by agent id. */
  wakes: Record<string, WakeEntry>;
}

export const INITIAL_SLEEP: SleepSnapshot = { wakes: {} };

const EVENT_WAKE_RESULT = "wake.result";
const EVENT_AGENT_STATUS = "agent.status";
const CODE_CONFLICT = "conflict";
const REQUEST_FAILED = "The request failed.";

/**
 * Owns the idle sleep setting and the wake of each asleep agent. The wake
 * entry of an agent lasts until the agent sleeps again or the person wakes it
 * again.
 */
export function createSleepController(client: CoreClient) {
  const service = createSleepService(client);
  let snapshot: SleepSnapshot = INITIAL_SLEEP;
  const listeners = new Set<(snapshot: SleepSnapshot) => void>();

  function publish(wakes: Record<string, WakeEntry>): void {
    snapshot = { wakes };
    for (const listener of listeners) listener(snapshot);
  }

  function setEntry(agentId: string, entry: WakeEntry): void {
    publish({ ...snapshot.wakes, [agentId]: entry });
  }

  function clearEntry(agentId: string): void {
    if (!(agentId in snapshot.wakes)) return;
    const rest = { ...snapshot.wakes };
    delete rest[agentId];
    publish(rest);
  }

  function describeWakeFailure(rejection: unknown): string {
    const failure = toCoreRequestError(rejection);
    if (failure.code === CODE_CONFLICT) return NO_LONGER_ASLEEP_TEXT;
    return failure.message || REQUEST_FAILED;
  }

  function handleEvent(event: ServerEvent): void {
    if (event.type === EVENT_WAKE_RESULT) {
      const result = readWakeResult(event);
      if (result !== null)
        setEntry(result.agent_id, { kind: "result", result });
      return;
    }
    if (event.type === EVENT_AGENT_STATUS && event.status === ASLEEP_STATUS) {
      clearEntry(String(event.agent_id));
    }
  }

  return {
    get snapshot(): SleepSnapshot {
      return snapshot;
    },

    /** Calls back on each change, starting with the current snapshot. */
    subscribe(listener: (snapshot: SleepSnapshot) => void): () => void {
      listeners.add(listener);
      listener(snapshot);
      return () => listeners.delete(listener);
    },

    /** Listens for the server's events. Call it once. */
    watch: async (): Promise<void> => {
      try {
        await client.watchEvents(handleEvent);
      } catch {
        // Without events the wake stays at "Waking"; the agent's own status
        // still moves.
      }
    },

    /** Wakes the agent; a wake that is already running is not sent twice. */
    wake: async (agentId: string): Promise<void> => {
      if (isWakeBusy(snapshot.wakes[agentId])) return;
      setEntry(agentId, { kind: "pending" });
      try {
        await service.wake(agentId);
      } catch (rejection) {
        setEntry(agentId, {
          kind: "failure",
          text: describeWakeFailure(rejection),
        });
      }
    },

    /** The setting as the dialog shows it. */
    readForm: async (): Promise<IdleSleepForm> =>
      readIdleSleepForm(await service.readIdleSleep()),

    /** Stores the form; resolves to the refusal text, or null when saved. */
    saveForm: async (form: IdleSleepForm): Promise<string | null> => {
      const problem = formProblem(form);
      if (problem !== null) return problem;
      try {
        await service.writeIdleSleep(settingValueOf(form));
        return null;
      } catch (rejection) {
        return toCoreRequestError(rejection).message || REQUEST_FAILED;
      }
    },

    /** The refusal text of a rejected read, for a dialog to show. */
    describeFailure: (rejection: unknown): string =>
      toCoreRequestError(rejection).message || REQUEST_FAILED,
  };
}

export type SleepController = ReturnType<typeof createSleepController>;
