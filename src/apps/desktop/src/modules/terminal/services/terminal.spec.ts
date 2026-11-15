// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import {
  dropQueryReplies,
  encodeBinaryInput,
  encodeInput,
  parseFrame,
} from "./terminal";

const ESC = "\u001b";
const AGENT_ID = "11111111-2222-4333-8444-555555555555";
const AGENT_BYTES = [
  0x11, 0x11, 0x11, 0x11, 0x22, 0x22, 0x43, 0x33, 0x84, 0x44, 0x55, 0x55, 0x55,
  0x55, 0x55, 0x55,
];

describe("dropQueryReplies", () => {
  test("drops a cursor-position reply", () => {
    expect(dropQueryReplies(`${ESC}[12;40R`)).toBe("");
    expect(dropQueryReplies(`${ESC}[?12;40R`)).toBe("");
  });

  test("drops primary and secondary device-attribute replies", () => {
    expect(dropQueryReplies(`${ESC}[?1;2c`)).toBe("");
    expect(dropQueryReplies(`${ESC}[?62;4c`)).toBe("");
    expect(dropQueryReplies(`${ESC}[>0;276;0c`)).toBe("");
    expect(dropQueryReplies(`${ESC}[>1c`)).toBe("");
  });

  test("keeps arrows, Enter, Ctrl+C, and normal text", () => {
    for (const keys of [
      `${ESC}[A`,
      `${ESC}[B`,
      `${ESC}[C`,
      `${ESC}[D`,
      `${ESC}OA`,
      "\r",
      "\u0003",
      "hello world",
      "c",
      "R",
      `${ESC}`,
      `${ESC}[1;5C`,
    ]) {
      expect(dropQueryReplies(keys)).toBe(keys);
    }
  });

  test("drops only the replies from a mixed chunk", () => {
    expect(dropQueryReplies(`ab${ESC}[3;7Rcd${ESC}[?1;2c\r`)).toBe("abcd\r");
  });
});

describe("parseFrame", () => {
  function frame(kind: number, text: string): Uint8Array {
    return Uint8Array.from([
      kind,
      ...AGENT_BYTES,
      ...new TextEncoder().encode(text),
    ]);
  }

  test("reads each kind with its agent and payload", () => {
    const snapshot = parseFrame(frame(1, "screen"));
    expect(snapshot?.kind).toBe("snapshot");
    expect(snapshot?.agentId).toBe(AGENT_ID);
    expect(new TextDecoder().decode(snapshot?.payload)).toBe("screen");
    expect(parseFrame(frame(2, "x"))?.kind).toBe("output");
    expect(parseFrame(frame(3, ""))?.kind).toBe("dropped");
  });

  test("ignores an unknown kind and a short frame", () => {
    expect(parseFrame(frame(9, "x"))).toBeNull();
    expect(parseFrame(Uint8Array.from([1, 2, 3]))).toBeNull();
    expect(parseFrame(new Uint8Array())).toBeNull();
  });
});

describe("input encoding", () => {
  test("encodes text as UTF-8 base64", () => {
    expect(encodeInput("hi\r")).toBe("aGkN");
    expect(encodeInput("é")).toBe("w6k=");
  });

  test("encodes binary input one byte per character", () => {
    expect(encodeBinaryInput("é\u0001")).toBe("6QE=");
  });
});
