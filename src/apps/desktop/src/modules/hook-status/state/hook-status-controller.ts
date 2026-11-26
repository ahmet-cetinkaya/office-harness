// SPDX-License-Identifier: AGPL-3.0-or-later

import { toCoreRequestError, type CoreClient } from "../../../core/api";
import { isSwitchOn, switchValueOf } from "../models/hook-status";
import { createHookStatusService } from "../services/hook-status-service";

const REQUEST_FAILED = "The request failed.";

/** Reads and stores the hook status setting (0024 FR-024). */
export function createHookStatusController(client: CoreClient) {
  const service = createHookStatusService(client);
  const describeFailure = (rejection: unknown): string =>
    toCoreRequestError(rejection).message || REQUEST_FAILED;
  return {
    /** Whether the setting is on, as the dialog shows it. */
    readIsOn: async (): Promise<boolean> => isSwitchOn(await service.read()),

    /** Stores the switch; resolves to the refusal text, or null when saved. */
    save: async (isOn: boolean): Promise<string | null> => {
      try {
        await service.write(switchValueOf(isOn));
        return null;
      } catch (rejection) {
        return describeFailure(rejection);
      }
    },

    /** The refusal text of a rejected read, for the dialog to show. */
    describeFailure,
  };
}

export type HookStatusController = ReturnType<
  typeof createHookStatusController
>;
