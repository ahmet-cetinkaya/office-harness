// SPDX-License-Identifier: AGPL-3.0-or-later

import type { CoreClient } from "../../../core/api";
import { parseFrame, type TerminalFrame } from "./terminal";

export type FrameListener = (frame: TerminalFrame) => void;

export interface TerminalFeed {
  /**
   * Delivers the frames of one agent to `listener` until the returned function
   * runs. Resolves once the core forwards frames, so a request that follows
   * cannot lose the snapshot.
   */
  subscribe(agentId: string, listener: FrameListener): Promise<() => void>;
}

/** Splits the core's one frame stream by agent. */
export function createTerminalFeed(client: CoreClient): TerminalFeed {
  const listeners = new Map<string, Set<FrameListener>>();
  let watching: Promise<void> | null = null;

  function dispatch(bytes: Uint8Array): void {
    const frame = parseFrame(bytes);
    if (frame === null) return;
    listeners.get(frame.agentId)?.forEach((listener) => listener(frame));
  }

  return {
    async subscribe(agentId, listener) {
      watching ??= client.watchTerminalFrames(dispatch);
      try {
        await watching;
      } catch (failure) {
        watching = null;
        throw failure;
      }
      const forAgent = listeners.get(agentId) ?? new Set();
      forAgent.add(listener);
      listeners.set(agentId, forAgent);
      return () => forAgent.delete(listener);
    },
  };
}

// The core keeps a forwarder for each watch call, so each client watches once.
const feeds = new WeakMap<CoreClient, TerminalFeed>();

export function terminalFeedFor(client: CoreClient): TerminalFeed {
  const known = feeds.get(client);
  if (known !== undefined) return known;
  const created = createTerminalFeed(client);
  feeds.set(client, created);
  return created;
}
