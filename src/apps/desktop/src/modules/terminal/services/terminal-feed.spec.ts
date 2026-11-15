// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import type { CoreClient } from "../../../core/api";
import { createTerminalFeed } from "./terminal-feed";
import type { TerminalFrame } from "./terminal";

const KIND_DROPPED = 3;
const AGENT_BYTES = Array.from({ length: 16 }, (_, index) => index + 1);
const AGENT_ID = "01020304-0506-0708-090a-0b0c0d0e0f10";

function feedWithPush() {
  let dispatch: (bytes: Uint8Array) => void = () => {};
  const client = {
    watchTerminalFrames: async (onFrame: (bytes: Uint8Array) => void) => {
      dispatch = onFrame;
    },
  } as unknown as CoreClient;
  return {
    feed: createTerminalFeed(client),
    push: (bytes: Uint8Array) => dispatch(bytes),
  };
}

describe("terminal feed", () => {
  test("a dropped frame without an agent reaches every listener", async () => {
    const { feed, push } = feedWithPush();
    const seen: TerminalFrame[] = [];
    await feed.subscribe(AGENT_ID, (frame) => seen.push(frame));

    push(Uint8Array.from([KIND_DROPPED, ...new Array<number>(16).fill(0)]));

    expect(seen.map((frame) => frame.kind)).toEqual(["dropped"]);
  });

  test("a dropped frame for one agent skips the others", async () => {
    const { feed, push } = feedWithPush();
    const seen: TerminalFrame[] = [];
    await feed.subscribe("other", (frame) => seen.push(frame));

    push(Uint8Array.from([KIND_DROPPED, ...AGENT_BYTES]));

    expect(seen).toEqual([]);
  });
});
