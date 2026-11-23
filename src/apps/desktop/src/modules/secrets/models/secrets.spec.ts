// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import {
  custodyText,
  firstSaveProblem,
  grantStatus,
  nameProblem,
  NAME_PROBLEM,
  readSecretsView,
  resetQuestion,
  storeLine,
  valueProblem,
  VALUE_HAS_NUL_PROBLEM,
  VALUE_TOO_LONG_PROBLEM,
  VALUE_TOO_SHORT_PROBLEM,
  type Secret,
} from "./secrets";

const SECRET: Secret = {
  name: "API_KEY",
  version: 2,
  updatedAt: 5,
  isUnreadable: false,
  grants: [],
};

describe("reading secret.list", () => {
  test("a ready store with a granted secret", () => {
    const view = readSecretsView({
      store: { state: "ready", custody: "key_file" },
      secrets: [
        {
          name: "API_KEY",
          version: 2,
          created_at: 1,
          updated_at: 5,
          unreadable: false,
          grants: [{ agent_id: "ada", last_read_version: 1, last_read_at: 4 }],
        },
      ],
    });

    expect(view.store).toEqual({ kind: "ready", custody: "key_file" });
    expect(view.secrets[0].grants).toEqual([
      { agentId: "ada", lastReadVersion: 1, lastReadAt: 4 },
    ]);
  });

  test("a locked store keeps its reason", () => {
    const view = readSecretsView({
      store: {
        state: "locked",
        custody: "os_keyring",
        locked_reason: "the keyring is locked",
      },
      secrets: [],
    });

    expect(view.store).toEqual({
      kind: "locked",
      custody: "os_keyring",
      reason: "the keyring is locked",
    });
  });

  test("an answer that is not as expected is an empty store", () => {
    expect(readSecretsView(null)).toEqual({
      store: { kind: "empty" },
      secrets: [],
    });
    expect(
      readSecretsView({ secrets: [{ name: "A_KEY" }, 7, { version: 1 }] })
        .secrets,
    ).toEqual([]);
  });
});

describe("the words of the dialog", () => {
  test("the store line names the custody, or the lock", () => {
    expect(storeLine({ kind: "empty" })).toBe("No secret saved yet.");
    expect(storeLine({ kind: "ready", custody: "os_keyring" })).toBe(
      "Stored with: OS keyring",
    );
    expect(storeLine({ kind: "ready", custody: "key_file" })).toBe(
      "Stored with: key file",
    );
    expect(
      storeLine({
        kind: "locked",
        custody: "os_keyring",
        reason: "the keyring is locked",
      }),
    ).toBe("Locked: the keyring is locked (OS keyring)");
    expect(custodyText("key_file")).toBe("key file");
  });

  test("a grant says what it read, in words", () => {
    const held = { agentId: "ada", lastReadAt: 4 };
    expect(grantStatus(SECRET, { ...held, lastReadVersion: null })).toBe(
      "not read yet",
    );
    expect(grantStatus(SECRET, { ...held, lastReadVersion: 2 })).toBe(
      "read version 2, current",
    );
    expect(grantStatus(SECRET, { ...held, lastReadVersion: 1 })).toBe(
      "read version 1, behind (current 2)",
    );
  });

  test("the reset question counts the secrets", () => {
    expect(resetQuestion(1)).toBe(
      "This removes the 1 saved secret and its key.",
    );
    expect(resetQuestion(3)).toBe(
      "This removes all 3 saved secrets and their key.",
    );
  });
});

describe("the rules of a name and a value", () => {
  test("a name is a letter A to Z, then letters, digits, and underscore", () => {
    for (const name of ["A", "OPENAI_API_KEY", "K9", "A".repeat(64)]) {
      expect(nameProblem(name)).toBeNull();
    }
    for (const name of ["", "key", "9KEY", "_KEY", "MY KEY", "A".repeat(65)]) {
      expect(nameProblem(name)).toBe(NAME_PROBLEM);
    }
  });

  test("a value is 8 to 8192 bytes without NUL, counted in bytes", () => {
    expect(valueProblem("12345678")).toBeNull();
    expect(valueProblem("1234567")).toBe(VALUE_TOO_SHORT_PROBLEM);
    // Four two-byte characters are 8 bytes.
    expect(valueProblem("çççç")).toBeNull();
    expect(valueProblem("x".repeat(8192))).toBeNull();
    expect(valueProblem("x".repeat(8193))).toBe(VALUE_TOO_LONG_PROBLEM);
    expect(valueProblem("12345\u00006789")).toBe(VALUE_HAS_NUL_PROBLEM);
  });

  test("the first problem is the name's, and no text holds the value", () => {
    expect(firstSaveProblem("key", "marker-value")).toBe(NAME_PROBLEM);
    const problem = firstSaveProblem("API_KEY", "marker");
    expect(problem).toBe(VALUE_TOO_SHORT_PROBLEM);
    expect(problem).not.toContain("marker");
    expect(firstSaveProblem("API_KEY", "marker-value")).toBeNull();
  });
});
