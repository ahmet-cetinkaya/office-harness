// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import { createInputQueue, MAX_REQUEST_BYTES } from "./input-queue";
import { textBytes } from "./terminal";

function decode(encoded: string): string {
  return atob(encoded);
}

function controlledSend() {
  const sent: string[] = [];
  const resolvers: (() => void)[] = [];
  const send = (encoded: string) => {
    sent.push(decode(encoded));
    return new Promise<void>((resolve) => resolvers.push(resolve));
  };
  return { sent, send, finishOldest: () => resolvers.shift()?.() };
}

const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

describe("input queue", () => {
  test("keeps one request in flight and joins what is typed meanwhile", async () => {
    const { sent, send, finishOldest } = controlledSend();
    const queue = createInputQueue(send, () => {});

    queue.push(textBytes("a"));
    queue.push(textBytes("b"));
    queue.push(textBytes("c"));
    expect(sent).toEqual(["a"]);

    finishOldest();
    await settle();
    expect(sent).toEqual(["a", "bc"]);
  });

  test("sends again once the queue went idle", async () => {
    const { sent, send, finishOldest } = controlledSend();
    const queue = createInputQueue(send, () => {});
    queue.push(textBytes("a"));
    finishOldest();
    await settle();

    queue.push(textBytes("b"));

    expect(sent).toEqual(["a", "b"]);
  });

  test("reports a failure and goes on with what waits", async () => {
    const failures: unknown[] = [];
    const sent: string[] = [];
    let calls = 0;
    const queue = createInputQueue(
      (encoded) => {
        sent.push(decode(encoded));
        calls += 1;
        return calls === 1
          ? Promise.reject(new Error("refused"))
          : Promise.resolve();
      },
      (failure) => failures.push(failure),
    );

    queue.push(textBytes("a"));
    queue.push(textBytes("b"));
    await settle();

    expect(failures).toHaveLength(1);
    expect(sent).toEqual(["a", "b"]);
  });
  test("keeps a paste larger than the engine's argument limit", () => {
    const { sent, send } = controlledSend();
    const queue = createInputQueue(send, () => {});
    const large = new Uint8Array(200_000).fill(0x61);

    queue.push(large);

    expect(sent).toHaveLength(1);
    expect(sent[0]).toHaveLength(200_000);
  });

  test("splits a paste above the request limit into requests in order", async () => {
    const { sent, send, finishOldest } = controlledSend();
    const queue = createInputQueue(send, () => {});
    const large = new Uint8Array(MAX_REQUEST_BYTES + 10).fill(0x62);

    queue.push(large);
    finishOldest();
    await settle();

    expect(sent.map((request) => request.length)).toEqual([
      MAX_REQUEST_BYTES,
      10,
    ]);
  });
});
