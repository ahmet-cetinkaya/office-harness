// SPDX-License-Identifier: AGPL-3.0-or-later

/** The rules of the server (0026/FR-016), checked here first so a refusal comes at once. */
export const SECRET_NAME_PATTERN = /^[A-Z][A-Z0-9_]{0,63}$/;
export const MIN_VALUE_BYTES = 8;
export const MAX_VALUE_BYTES = 8192;
export const MAX_SECRETS = 100;

export const NAME_PROBLEM =
  "The name is 1 to 64 characters: it starts with a letter A to Z and holds only the letters A to Z, digits, and underscore.";
export const VALUE_TOO_SHORT_PROBLEM = `The value is at least ${MIN_VALUE_BYTES} bytes.`;
export const VALUE_TOO_LONG_PROBLEM = `The value is at most ${MAX_VALUE_BYTES} bytes.`;
export const VALUE_HAS_NUL_PROBLEM = "The value holds a NUL character.";
export const TOO_MANY_PROBLEM = `At most ${MAX_SECRETS} secrets.`;

export type Custody = "os_keyring" | "key_file";

export type StoreState =
  | { kind: "empty" }
  | { kind: "ready"; custody: Custody }
  | { kind: "locked"; custody: Custody; reason: string };

export interface SecretGrant {
  agentId: string;
  lastReadVersion: number | null;
  lastReadAt: number | null;
}

export interface Secret {
  name: string;
  version: number;
  updatedAt: number;
  /** The sealed record does not open: the person can replace the value. */
  isUnreadable: boolean;
  grants: SecretGrant[];
}

export interface SecretsView {
  store: StoreState;
  secrets: Secret[];
}

export const EMPTY_VIEW: SecretsView = {
  store: { kind: "empty" },
  secrets: [],
};

function asRecord(value: unknown): Record<string, unknown> {
  return typeof value === "object" && value !== null
    ? (value as Record<string, unknown>)
    : {};
}

function numberOrNull(value: unknown): number | null {
  return typeof value === "number" ? value : null;
}

function custodyOf(value: unknown): Custody {
  return value === "key_file" ? "key_file" : "os_keyring";
}

function readStore(value: unknown): StoreState {
  const store = asRecord(value);
  if (store.state === "ready") {
    return { kind: "ready", custody: custodyOf(store.custody) };
  }
  if (store.state === "locked") {
    return {
      kind: "locked",
      custody: custodyOf(store.custody),
      reason:
        typeof store.locked_reason === "string" ? store.locked_reason : "",
    };
  }
  return { kind: "empty" };
}

function readGrant(value: unknown): SecretGrant | null {
  const grant = asRecord(value);
  if (typeof grant.agent_id !== "string") return null;
  return {
    agentId: grant.agent_id,
    lastReadVersion: numberOrNull(grant.last_read_version),
    lastReadAt: numberOrNull(grant.last_read_at),
  };
}

function readSecret(value: unknown): Secret | null {
  const secret = asRecord(value);
  if (typeof secret.name !== "string" || typeof secret.version !== "number") {
    return null;
  }
  const grants = Array.isArray(secret.grants) ? secret.grants : [];
  return {
    name: secret.name,
    version: secret.version,
    updatedAt: numberOrNull(secret.updated_at) ?? 0,
    isUnreadable: secret.unreadable === true,
    grants: grants
      .map(readGrant)
      .filter((grant): grant is SecretGrant => grant !== null),
  };
}

/** Reads the answer of `secret.list`; a part that is not as expected is left out. */
export function readSecretsView(result: unknown): SecretsView {
  const answer = asRecord(result);
  const secrets = Array.isArray(answer.secrets) ? answer.secrets : [];
  return {
    store: readStore(answer.store),
    secrets: secrets
      .map(readSecret)
      .filter((secret): secret is Secret => secret !== null),
  };
}

export function custodyText(custody: Custody): string {
  return custody === "key_file" ? "key file" : "OS keyring";
}

/** The line at the top of the dialog: where the store key lives, as text. */
export function storeLine(store: StoreState): string {
  switch (store.kind) {
    case "empty":
      return "No secret saved yet.";
    case "ready":
      return `Stored with: ${custodyText(store.custody)}`;
    case "locked":
      return `Locked: ${store.reason} (${custodyText(store.custody)})`;
  }
}

/** What one grant has read, in words and not in colour alone (FR-017). */
export function grantStatus(secret: Secret, grant: SecretGrant): string {
  if (grant.lastReadVersion === null) return "not read yet";
  if (grant.lastReadVersion === secret.version) {
    return `read version ${grant.lastReadVersion}, current`;
  }
  return `read version ${grant.lastReadVersion}, behind (current ${secret.version})`;
}

export function nameProblem(name: string): string | null {
  return SECRET_NAME_PATTERN.test(name) ? null : NAME_PROBLEM;
}

export function valueProblem(value: string): string | null {
  if (value.includes("\u0000")) return VALUE_HAS_NUL_PROBLEM;
  const bytes = new TextEncoder().encode(value).length;
  if (bytes < MIN_VALUE_BYTES) return VALUE_TOO_SHORT_PROBLEM;
  if (bytes > MAX_VALUE_BYTES) return VALUE_TOO_LONG_PROBLEM;
  return null;
}

/** The first problem of a name and a value, or null. The text never holds the value. */
export function firstSaveProblem(name: string, value: string): string | null {
  return nameProblem(name) ?? valueProblem(value);
}

export function resetQuestion(count: number): string {
  return count === 1
    ? "This removes the 1 saved secret and its key."
    : `This removes all ${count} saved secrets and their key.`;
}
