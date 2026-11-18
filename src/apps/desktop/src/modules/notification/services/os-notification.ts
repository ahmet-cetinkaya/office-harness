// SPDX-License-Identifier: AGPL-3.0-or-later

import type { ActionPerformed } from "@tauri-apps/plugin-notification";
import {
  isPermissionGranted,
  onAction,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";

/**
 * The OS notifications the webview sends (0003, D7). The plugin's imports
 * alone talk to `window.__TAURI_INTERNALS__`, so the service is a no-op where
 * they are missing, as in a browser tab or the tests.
 */
export interface OsNotificationService {
  /** True when the OS may show one. Asks once, at the first use. */
  isAllowed(): Promise<boolean>;
  /** Shows one notification, tagged with the agent it came from. */
  show(agentId: string, title: string, body: string): void;
  /** Calls back with the agent id of a tapped notification. */
  onTap(onTapAgent: (agentId: string) => void): Promise<void>;
}

/** The agent id a performed action carries, or null when the OS drops it. */
export function agentIdOf(performed: ActionPerformed): string | null {
  const agentId = performed.notification?.extra?.agent_id;
  return typeof agentId === "string" ? agentId : null;
}

export function createOsNotificationService(): OsNotificationService {
  const available =
    typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
  let asked = false;
  let allowed = false;

  async function isAllowed(): Promise<boolean> {
    if (!available) return false;
    if (asked) return allowed;
    asked = true;
    try {
      allowed = await isPermissionGranted();
      if (!allowed) allowed = (await requestPermission()) === "granted";
    } catch {
      // A refused ask behaves like a denial: the list still works (D7).
      allowed = false;
    }
    return allowed;
  }

  return {
    isAllowed,

    show(agentId: string, title: string, body: string): void {
      if (!allowed) return;
      try {
        sendNotification({ title, body, extra: { agent_id: agentId } });
      } catch (failure) {
        console.warn("office-harness could not notify:", failure);
      }
    },

    async onTap(onTapAgent: (agentId: string) => void): Promise<void> {
      if (!available) return;
      try {
        await onAction((performed) => {
          if (performed.actionId !== "tap") return;
          const agentId = agentIdOf(performed);
          if (agentId !== null) onTapAgent(agentId);
        });
      } catch (failure) {
        console.warn(
          "office-harness could not watch notification taps:",
          failure,
        );
      }
    },
  };
}
