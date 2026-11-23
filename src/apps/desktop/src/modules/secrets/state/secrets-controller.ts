// SPDX-License-Identifier: AGPL-3.0-or-later

import {
  toCoreRequestError,
  type CoreClient,
  type ServerEvent,
} from "../../../core/api";
import {
  EMPTY_VIEW,
  firstSaveProblem,
  type SecretsView,
} from "../models/secrets";
import { createSecretsService } from "../services/secrets-service";

export interface SecretsSnapshot {
  view: SecretsView;
  /** True once a read answered, so the dialog can tell loading from empty. */
  isLoaded: boolean;
  /** The refusal text of the last read, or null. */
  loadFailure: string | null;
}

export const INITIAL_SECRETS: SecretsSnapshot = {
  view: EMPTY_VIEW,
  isLoaded: false,
  loadFailure: null,
};

const EVENT_SECRET_CHANGED = "secret.changed";
const EVENT_RESYNC = "client.resync";
const REQUEST_FAILED = "The request failed.";

/** A request's refusal text, or null when the request went through. */
export type FailureText = string | null;

/**
 * Owns the list of secrets as the person sees it: names, versions, grants,
 * and the state of the store. It never keeps a value: a typed value is an
 * argument of `save` and goes to the server in the same call. The server
 * sends `secret.changed` after each change, and the list is read again.
 */
export function createSecretsController(client: CoreClient) {
  const service = createSecretsService(client);
  let snapshot: SecretsSnapshot = INITIAL_SECRETS;
  const listeners = new Set<(snapshot: SecretsSnapshot) => void>();
  // An event is newer than any read that is still out.
  let newestRead = 0;

  function publish(changes: Partial<SecretsSnapshot>): void {
    snapshot = { ...snapshot, ...changes };
    for (const listener of listeners) listener(snapshot);
  }

  function describe(rejection: unknown): string {
    return toCoreRequestError(rejection).message || REQUEST_FAILED;
  }

  async function submit(work: () => Promise<unknown>): Promise<FailureText> {
    try {
      await work();
      return null;
    } catch (rejection) {
      return describe(rejection);
    }
  }

  async function load(): Promise<void> {
    const readId = ++newestRead;
    try {
      const view = await service.list();
      if (readId === newestRead) {
        publish({ view, isLoaded: true, loadFailure: null });
      }
    } catch (rejection) {
      if (readId === newestRead) publish({ loadFailure: describe(rejection) });
    }
  }

  function handleEvent(event: ServerEvent): void {
    if (event.type === EVENT_SECRET_CHANGED || event.type === EVENT_RESYNC) {
      void load();
    }
  }

  return {
    get snapshot(): SecretsSnapshot {
      return snapshot;
    },

    /** Calls back on each change, starting with the current snapshot. */
    subscribe(listener: (snapshot: SecretsSnapshot) => void): () => void {
      listeners.add(listener);
      listener(snapshot);
      return () => listeners.delete(listener);
    },

    /** Listens for the server's events. Call it once. */
    watch: async (): Promise<void> => {
      try {
        await client.watchEvents(handleEvent);
      } catch {
        // Without events the list is read again after each of the person's
        // own changes, which is what `submitThenLoad` does.
      }
    },

    load,

    /** Saves a new secret or replaces the value of one; null when saved. */
    save: async (name: string, value: string): Promise<FailureText> => {
      const problem = firstSaveProblem(name, value);
      if (problem !== null) return problem;
      return submit(() => service.save(name, value));
    },

    remove: (name: string): Promise<FailureText> =>
      submit(() => service.remove(name)),

    grant: (name: string, agentId: string): Promise<FailureText> =>
      submit(() => service.grant(name, agentId)),

    revoke: (name: string, agentId: string): Promise<FailureText> =>
      submit(() => service.revoke(name, agentId)),

    reset: (confirmCount: number): Promise<FailureText> =>
      submit(() => service.reset(confirmCount)),

    describeFailure: describe,
  };
}

export type SecretsController = ReturnType<typeof createSecretsController>;
