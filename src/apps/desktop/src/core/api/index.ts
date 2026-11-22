// SPDX-License-Identifier: AGPL-3.0-or-later

export type { CoreClient, ExportOutcome, ServerEvent } from "./core-client";
export {
  CORE_UNAVAILABLE,
  CoreRequestError,
  NOT_CONNECTED,
  toCoreRequestError,
} from "./core-request-error";
export type { ApiError } from "./generated/ApiError";
export type { ConnectionState } from "./generated/ConnectionState";
export {
  createTauriCoreClient,
  setWindowTitleCount,
} from "./tauri-core-client";
