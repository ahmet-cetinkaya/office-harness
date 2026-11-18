// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import {
  countAgentsText,
  deleteQuestionText,
  roleKindText,
  stopQuestionText,
} from "./confirmations";

describe("role text and refusals", () => {
  test("roles read as words", () => {
    expect(roleKindText("ceo")).toBe("CEO");
    expect(roleKindText("lead")).toBe("team lead");
    expect(roleKindText("member")).toBe("member");
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

  test("a stop offers both outcomes in plain words (FR-007)", () => {
    const question = stopQuestionText("Bo", 2);
    expect(question.title).toBe("Stop Bo?");
    expect(question.body).toContain("Only Bo stops");
    expect(question.body).toContain("2 agents below Bo keep running");
    expect(question.body).not.toContain("include_descendants");
  });

  test("a stop without descendants names the single outcome (FR-007)", () => {
    expect(stopQuestionText("Bo", 0).body).toBe(
      "The agent's CLI is ended and the session stops.",
    );
  });
});
