// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import {
  countAgentsText,
  deleteQuestionText,
  roleKindText,
  roleRefusalText,
  stopQuestionText,
} from "./confirmations";

describe("role text and refusals", () => {
  test("roles read as words", () => {
    expect(roleKindText("ceo")).toBe("CEO");
    expect(roleKindText("lead")).toBe("team lead");
    expect(roleKindText("member")).toBe("member");
  });

  test("a refusal names the roles allowed at the parent (FR-003)", () => {
    expect(roleRefusalText("member", "ceo")).toBe(
      "a member cannot sit under a CEO; allowed under a CEO: team lead",
    );
    expect(roleRefusalText("lead", "lead")).toBe(
      "a team lead cannot sit under a team lead; allowed under a team lead: member",
    );
    expect(roleRefusalText("member", "member")).toBe(
      "a member cannot sit under a member; allowed under a member: nothing",
    );
  });
});

describe("counted questions", () => {
  test("counts read as spoken text", () => {
    expect(countAgentsText(1)).toBe("1 agent");
    expect(countAgentsText(2)).toBe("2 agents");
    expect(countAgentsText(0)).toBe("0 agents");
  });

  test("a delete states the counted descendants (FR-006)", () => {
    const question = deleteQuestionText("Bo", 2);
    expect(question.title).toBe("Delete Bo?");
    expect(question.body).toContain("2 agents");
    expect(question.body).toContain("3 agents in all");
  });

  test("a delete without descendants names only the agent", () => {
    expect(deleteQuestionText("Bo", 0).body).toBe(
      "This deletes Bo. The action record keeps the entries.",
    );
  });

  test("a stop offers to stop the descendants too (FR-007)", () => {
    const question = stopQuestionText("Bo", 2);
    expect(question.title).toBe("Stop Bo?");
    expect(question.body).toContain("2 agents below Bo keep running");
  });

  test("a stop without descendants is the plain stop question", () => {
    expect(stopQuestionText("Bo", 0).body).toBe(
      "The agent's CLI is ended and the session stops.",
    );
  });
});
