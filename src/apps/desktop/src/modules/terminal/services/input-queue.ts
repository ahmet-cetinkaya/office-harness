// SPDX-License-Identifier: AGPL-3.0-or-later

import { encodeBytes } from "./terminal";

export interface InputQueue {
  /** Sends `bytes` now, or after the request in flight, in typing order. */
  push(bytes: Uint8Array): void;
}

/**
 * Keeps one input request in flight. Bytes that come meanwhile wait and go
 * out as one request, so a slow server sees a bounded number of requests.
 */
export function createInputQueue(
  send: (encoded: string) => Promise<unknown>,
  onFailure: (failure: unknown) => void,
): InputQueue {
  let waiting: number[] = [];
  let isSending = false;

  function sendWaiting(): void {
    isSending = true;
    const batch = Uint8Array.from(waiting);
    waiting = [];
    send(encodeBytes(batch))
      .catch(onFailure)
      .finally(() => {
        isSending = false;
        if (waiting.length > 0) sendWaiting();
      });
  }

  return {
    push(bytes) {
      // One push per byte: spreading a large paste into one call passes
      // more arguments than JavaScriptCore allows.
      for (const byte of bytes) waiting.push(byte);
      if (!isSending) sendWaiting();
    },
  };
}
