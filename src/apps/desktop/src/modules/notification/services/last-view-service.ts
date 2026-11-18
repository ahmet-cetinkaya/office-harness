// SPDX-License-Identifier: AGPL-3.0-or-later

import type { CoreClient } from "../../../core/api";
import { parseLastView, type LastView } from "../models/last-view";

const LAST_VIEW_KEY = "last_view";

/**
 * The `last_view` setting (FR-008, D6). The server stores the JSON text; a
 * value this app cannot parse is ignored, so the first visit's view stands.
 */
export async function readLastView(
  client: CoreClient,
): Promise<LastView | null> {
  try {
    const setting = (await client.sendRequest("settings.get", {
      key: LAST_VIEW_KEY,
    })) as { value?: unknown } | null | undefined;
    return parseLastView(setting?.value);
  } catch {
    return null;
  }
}

/** Stores the view. A failure is quiet: the next change stores again. */
export async function writeLastView(
  client: CoreClient,
  view: LastView,
): Promise<void> {
  await client.sendRequest("settings.set", {
    key: LAST_VIEW_KEY,
    value: JSON.stringify(view),
  });
}
