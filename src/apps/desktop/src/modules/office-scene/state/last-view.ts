// SPDX-License-Identifier: AGPL-3.0-or-later

import type { CoreClient } from "../../../core/api";
import type { LastView } from "../../notification";

const WRITE_INTERVAL_MS = 1_000;
const SETTING_KEY = "last_view";

/**
 * Writes `last_view` through `settings.set` at most once per second (D6):
 * the first write of a window goes out at once, a write inside the window
 * is held and flushed at the window's end, so the final state always
 * lands. This is the scene-owned writer standing in for the 0003 T7 panel
 * writer, which is not on `main` yet (research R1); when that lands, both
 * call sites unify behind it and this module folds away.
 */
export function createLastViewWriter(
  client: CoreClient,
  now: () => number = () => Date.now(),
) {
  let lastSentAt = 0;
  let pending: LastView | null = null;
  let timer: ReturnType<typeof setTimeout> | null = null;

  function send(view: LastView): void {
    void client.sendRequest("settings.set", {
      key: SETTING_KEY,
      value: JSON.stringify(view),
    });
  }

  function flush(): void {
    timer = null;
    if (pending === null) return;
    const view = pending;
    pending = null;
    lastSentAt = now();
    send(view);
  }

  return {
    /** Records the current view; writes now or schedules the window's tail. */
    write(view: LastView): void {
      if (timer !== null) {
        pending = view;
        return;
      }
      const time = now();
      if (time - lastSentAt >= WRITE_INTERVAL_MS) {
        lastSentAt = time;
        send(view);
        return;
      }
      pending = view;
      timer = setTimeout(flush, WRITE_INTERVAL_MS - (time - lastSentAt));
    },
    /** Sends a held write at once (the app-close moment, D6). */
    close(): void {
      if (timer !== null) {
        clearTimeout(timer);
        timer = null;
      }
      flush();
    },
  };
}
