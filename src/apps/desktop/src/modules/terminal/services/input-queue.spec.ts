// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, test } from "bun:test";
import { createInputQueue } from "./input-queue";
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
});
