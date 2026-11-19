// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import {
  announcementOfChange,
  automaticHoldText,
  holdOfAgent,
  NO_HOLDS,
  readHoldState,
  type Hold,
} from "./hold";
import { LIMIT_INVALID_MESSAGE, limitProblem } from "./limits";

const ada: Hold = { agent_id: "ada", set_by: "user", since: 1 };
const bo: Hold = {
  agent_id: "bo",
  set_by: "office",
  reason: 'the limit "agents created by one agent"',
  since: 2,
};
const OFFICE: Hold = { set_by: "user", since: 3 };

const nameOf = (agentId: string) => agentId.toUpperCase();

describe("hold state", () => {
  test("a list answer becomes the office hold and the agent holds", () => {
    expect(readHoldState({ office: OFFICE, agents: [ada, bo] })).toEqual({
      office: OFFICE,
      agents: [ada, bo],
    });
  });

  test("an answer without holds, or not an object, is no hold", () => {
    expect(readHoldState({ agents: [] })).toEqual(NO_HOLDS);
    expect(readHoldState(null)).toEqual(NO_HOLDS);
    expect(readHoldState("x")).toEqual(NO_HOLDS);
    expect(readHoldState({ office: 7, agents: [{ set_by: 1 }, ada] })).toEqual({
      office: null,
      agents: [ada],
    });
  });

  test("an agent's hold is found by its id", () => {
    const state = { office: null, agents: [ada, bo] };
    expect(holdOfAgent(state, "bo")).toBe(bo);
    expect(holdOfAgent(state, "cy")).toBeNull();
  });

  test("only a hold by the office shows as automatic, with its reason", () => {
    expect(automaticHoldText(ada)).toBeNull();
    expect(automaticHoldText(null)).toBeNull();
    expect(automaticHoldText(bo)).toBe(
      'Held automatically: the limit "agents created by one agent"',
    );
    expect(automaticHoldText({ ...bo, reason: null })).toBe(
      "Held automatically",
    );
  });
});

describe("hold announcements", () => {
  test("holding and releasing the office are announced", () => {
    const held = { office: OFFICE, agents: [] };
    expect(announcementOfChange(NO_HOLDS, held, nameOf)).toBe("Office on hold");
    expect(announcementOfChange(held, NO_HOLDS, nameOf)).toBe(
      "Office released from hold",
    );
  });

  test("holding and releasing an agent name the agent", () => {
    const held = { office: null, agents: [ada] };
    expect(announcementOfChange(NO_HOLDS, held, nameOf)).toBe("ADA on hold");
    expect(announcementOfChange(held, NO_HOLDS, nameOf)).toBe(
      "ADA released from hold",
    );
  });

  test("an agent that the app does not know is not announced", () => {
    const known = (agentId: string) => (agentId === "ada" ? "ADA" : null);
    const held = { office: null, agents: [ada] };
    expect(announcementOfChange(held, NO_HOLDS, () => null)).toBeNull();
    expect(announcementOfChange(NO_HOLDS, held, () => null)).toBeNull();
    expect(
      announcementOfChange(held, { office: OFFICE, agents: [] }, known),
    ).toBe("Office on hold. ADA released from hold");
  });

  test("a state that changed nothing announces nothing", () => {
    const held = { office: OFFICE, agents: [ada] };
    expect(announcementOfChange(held, held, nameOf)).toBeNull();
  });
});

describe("limit values", () => {
  test("whole numbers from 1 to 1000 are limits", () => {
    for (const text of ["1", "10", " 3 ", "1000"]) {
      expect(limitProblem(text)).toBeNull();
    }
  });

  test("zero, negatives, text, fractions, and big numbers are refused with the server's words", () => {
    for (const text of ["0", "-1", "abc", "", "2.5", "1001", "1e3"]) {
      expect(limitProblem(text)).toBe(LIMIT_INVALID_MESSAGE);
    }
    expect(LIMIT_INVALID_MESSAGE).toBe(
      "a limit is a whole number of at least 1 and at most 1000",
    );
  });
});
