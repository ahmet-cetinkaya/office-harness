// SPDX-License-Identifier: AGPL-3.0-or-later

import {
  CORE_UNAVAILABLE,
  NOT_CONNECTED,
  toCoreRequestError,
  type ConnectionState,
  type CoreClient,
} from "../api";

export const RESTART_SERVER_REQUEST = "server.restart";

/** What the user asked the app to do about the server, and how it went. */
export type ActionProgress =
  | { phase: "idle" }
  | { phase: "running" }
  | { phase: "failed"; message: string };

export interface ConnectionSnapshot {
  state: ConnectionState;
  action: ActionProgress;
  /** Set when the core cannot be reached at all, for example in a browser. */
  coreError: string | null;
}

const IDLE: ActionProgress = { phase: "idle" };

export function createConnectionController(
  client: CoreClient,
  onChange: (snapshot: ConnectionSnapshot) => void,
) {
  let snapshot: ConnectionSnapshot = {
    state: { status: "loading" },
    action: IDLE,
    coreError: null,
  };

  function update(changes: Partial<ConnectionSnapshot>): void {
    snapshot = { ...snapshot, ...changes };
    onChange(snapshot);
  }

  /** An action's outcome belongs to the view it was started from. */
  function showState(state: ConnectionState): void {
    update(
      state.status === snapshot.state.status
        ? { state }
        : { state, action: IDLE },
    );
  }

  async function runAction(
    work: () => Promise<void>,
    describeFailure: (code: string, message: string) => string | null,
  ): Promise<void> {
    update({ action: { phase: "running" } });
    try {
      await work();
      update({ action: IDLE });
    } catch (rejection) {
      const { code, message } = toCoreRequestError(rejection);
      const failure = describeFailure(code, message);
      update({
        action: failure === null ? IDLE : { phase: "failed", message: failure },
      });
    }
  }

  return {
    async start(): Promise<void> {
      try {
        await client.watchConnection(showState);
      } catch (rejection) {
        update({ coreError: toCoreRequestError(rejection).message });
      }
    },

    startServer: () =>
      runAction(
        () => client.startServer(),
        (_code, message) => message,
      ),

    /**
     * The server closes the connection as it restarts, so "not connected"
     * after the request went out means that the restart began.
     */
    restartServer: () =>
      runAction(
        async () => {
          await client.sendRequest(RESTART_SERVER_REQUEST);
        },
        (code, message) => (code === NOT_CONNECTED ? null : message),
      ),

    async retry(): Promise<void> {
      try {
        await client.retryConnection();
      } catch (rejection) {
        const { code, message } = toCoreRequestError(rejection);
        if (code === CORE_UNAVAILABLE) update({ coreError: message });
      }
    },
  };
}

export type ConnectionController = ReturnType<
  typeof createConnectionController
>;
