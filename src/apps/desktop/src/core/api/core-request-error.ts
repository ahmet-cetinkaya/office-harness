// SPDX-License-Identifier: AGPL-3.0-or-later

import type { ApiError } from "./generated/ApiError";
import type { FieldFailure } from "./generated/FieldFailure";

export const CORE_UNAVAILABLE = "core_unavailable";
export const NOT_CONNECTED = "not_connected";

/** A failed command: the server's error code or one of the core's own. */
export class CoreRequestError extends Error {
  readonly code: string;
  /** The failing fields of an `invalid` answer, each with its rule. */
  readonly fields: FieldFailure[] | undefined;

  constructor(code: string, message: string, fields?: FieldFailure[]) {
    super(message);
    this.name = "CoreRequestError";
    this.code = code;
    this.fields = fields;
  }
}

function isApiError(value: unknown): value is ApiError {
  if (typeof value !== "object" || value === null) return false;
  const candidate = value as Record<string, unknown>;
  return (
    typeof candidate.code === "string" && typeof candidate.message === "string"
  );
}

/** Tauri rejects with the serialized Rust error, or with text when the call
 * never reached a command. */
export function toCoreRequestError(rejection: unknown): CoreRequestError {
  if (rejection instanceof CoreRequestError) return rejection;
  if (isApiError(rejection)) {
    return new CoreRequestError(
      rejection.code,
      rejection.message,
      rejection.fields,
    );
  }
  const message =
    rejection instanceof Error ? rejection.message : String(rejection);
  return new CoreRequestError(CORE_UNAVAILABLE, message);
}
