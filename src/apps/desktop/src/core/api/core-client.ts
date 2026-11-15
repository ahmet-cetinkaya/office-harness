// SPDX-License-Identifier: AGPL-3.0-or-later

import type { ConnectionState } from "./generated/ConnectionState";

/** A message from the server, as the core forwards it. Its `type` names it. */
export type ServerEvent = { type: string } & Record<string, unknown>;

/**
 * The webview's only way to reach the office server. The Rust core owns the
 * socket and the token; this interface has no place for either.
 */
export interface CoreClient {
  /** Calls back with the current state, then with each change. */
  watchConnection(onState: (state: ConnectionState) => void): Promise<void>;
  watchEvents(onEvent: (event: ServerEvent) => void): Promise<void>;
  /**
   * Calls back with each binary terminal frame (contract: "Terminal data
   * frames"). The core forwards them for every attached agent.
   */
  watchTerminalFrames(onFrame: (frame: Uint8Array) => void): Promise<void>;
  sendRequest(kind: string, fields?: Record<string, unknown>): Promise<unknown>;
  /** Resolves once the new server answers; rejects when it does not. */
  startServer(): Promise<void>;
  /** Skips the wait before the next connection attempt. */
  retryConnection(): Promise<void>;
}
