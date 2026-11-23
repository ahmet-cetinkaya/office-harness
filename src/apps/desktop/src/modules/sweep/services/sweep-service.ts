// SPDX-License-Identifier: AGPL-3.0-or-later

import type { CoreClient } from "../../../core/api";
import { readSweepState, type SweepState } from "../models/sweep";

/** The typed face of the `sweep.state` request. */
export function createSweepService(client: CoreClient) {
  return {
    state: async (): Promise<SweepState | null> =>
      readSweepState(await client.sendRequest("sweep.state")),
  };
}
