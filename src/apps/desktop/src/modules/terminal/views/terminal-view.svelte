<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import { FitAddon } from "@xterm/addon-fit";
  import { WebglAddon } from "@xterm/addon-webgl";
  import { Terminal } from "@xterm/xterm";
  import "@xterm/xterm/css/xterm.css";
  import { onMount } from "svelte";
  import FailureMessage from "$shared/ui/failure-message.svelte";
  import { toCoreRequestError, type CoreClient } from "../../../core/api";
  import { matchesShortcut } from "../../../core/shortcuts";
  import {
    loadFocusShortcut,
    type FocusShortcut,
  } from "../services/focus-shortcut";
  import {
    dropQueryReplies,
    encodeBinaryInput,
    encodeInput,
    type TerminalFrame,
  } from "../services/terminal";
  import { terminalFeedFor } from "../services/terminal-feed";

  interface Props {
    client: CoreClient;
    agentId: string;
    agentName: string;
  }

  let { client, agentId, agentName }: Props = $props();

  const RESIZE_DEBOUNCE_MS = 100;
  const SCROLLBACK_LINES = 10_000;
  // Light grey on near black is 15:1, well above the 4.5:1 that text needs.
  const THEME = { background: "#111827", foreground: "#f3f4f6" };

  let host: HTMLDivElement;
  let heading: HTMLHeadingElement;
  let focusShortcut = $state<FocusShortcut | null>(null);
  let hasDroppedOutput = $state(false);
  let problem = $state<string | null>(null);

  onMount(() => {
    const terminal = new Terminal({
      theme: THEME,
      scrollback: SCROLLBACK_LINES,
      // Puts the screen text in the page for screen readers.
      screenReaderMode: true,
    });
    const fit = new FitAddon();
    let isDisposed = false;
    let isTypingKey = false;
    // The server refuses a resize before the attach, which sends the size.
    let isAttached = false;
    let stopListening = () => {};
    let resizeTimer: ReturnType<typeof setTimeout> | undefined;

    function report(request: Promise<unknown>): void {
      request.catch((failure: unknown) => {
        problem = toCoreRequestError(failure).message;
      });
    }

    function sendInput(encoded: string): void {
      report(
        client.sendRequest("terminal.input", {
          agent_id: agentId,
          data: encoded,
        }),
      );
    }

    function showFrame(frame: TerminalFrame): void {
      if (frame.kind === "dropped") {
        hasDroppedOutput = true;
        return;
      }
      if (frame.kind === "snapshot") terminal.reset();
      terminal.write(frame.payload);
    }

    // The only key that does not reach the agent (FR-017).
    function handleKey(event: KeyboardEvent): boolean {
      if (
        focusShortcut !== null &&
        matchesShortcut(focusShortcut.shortcut, event)
      ) {
        if (event.type === "keydown") heading.focus();
        return false;
      }
      // xterm.js emits a keystroke's data inside this call. Only data that
      // comes later is an automatic reply, which dropQueryReplies removes.
      isTypingKey = true;
      queueMicrotask(() => (isTypingKey = false));
      return true;
    }

    function loadRenderer(): void {
      try {
        const webgl = new WebglAddon();
        webgl.onContextLoss(() => webgl.dispose());
        terminal.loadAddon(webgl);
      } catch {
        // Without WebGL the default renderer stays, which is fine.
      }
    }

    function scheduleFit(): void {
      clearTimeout(resizeTimer);
      resizeTimer = setTimeout(() => fit.fit(), RESIZE_DEBOUNCE_MS);
    }

    async function attach(): Promise<void> {
      focusShortcut = await loadFocusShortcut(client);
      const release = await terminalFeedFor(client).subscribe(
        agentId,
        showFrame,
      );
      if (isDisposed) {
        release();
        return;
      }
      stopListening = release;
      fit.fit();
      isAttached = true;
      await client.sendRequest("terminal.attach", {
        agent_id: agentId,
        cols: terminal.cols,
        rows: terminal.rows,
      });
      terminal.focus();
    }

    terminal.loadAddon(fit);
    terminal.open(host);
    terminal.textarea?.setAttribute("aria-label", `Terminal of ${agentName}`);
    loadRenderer();
    terminal.attachCustomKeyEventHandler(handleKey);
    terminal.onData((text) => {
      const forwarded = isTypingKey ? text : dropQueryReplies(text);
      if (forwarded !== "") sendInput(encodeInput(forwarded));
    });
    terminal.onBinary((text) => sendInput(encodeBinaryInput(text)));
    terminal.onResize(({ cols, rows }) => {
      if (!isAttached) return;
      report(
        client.sendRequest("terminal.resize", {
          agent_id: agentId,
          cols,
          rows,
        }),
      );
    });
    const observer = new ResizeObserver(scheduleFit);
    observer.observe(host);
    report(attach());

    return () => {
      isDisposed = true;
      clearTimeout(resizeTimer);
      observer.disconnect();
      stopListening();
      // The server detaches a client that is gone, so a failure here is moot.
      client
        .sendRequest("terminal.detach", { agent_id: agentId })
        .catch(() => {});
      terminal.dispose();
    };
  });
</script>

<section aria-labelledby="terminal-heading" class="flex flex-col gap-2">
  <h3
    id="terminal-heading"
    bind:this={heading}
    tabindex="-1"
    class="text-lg font-semibold text-neutral-900 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-700"
  >
    Terminal
  </h3>
  <p class="text-sm text-neutral-900">
    {#if focusShortcut !== null}
      Every key goes to the agent, except
      <kbd class="font-mono font-medium">{focusShortcut.label}</kbd>, which
      moves focus out of the terminal back to the app.
    {:else}
      Opening the terminal.
    {/if}
  </p>
  {#if hasDroppedOutput}
    <p
      role="status"
      class="rounded-md border border-neutral-500 bg-neutral-50 p-3 text-sm text-neutral-900"
    >
      <strong>Some output was dropped.</strong>
      The terminal fell behind, so older output is missing from the screen.
    </p>
  {/if}
  {#if problem !== null}
    <FailureMessage lead="The terminal has a problem." message={problem} />
  {/if}
  <div
    bind:this={host}
    role="group"
    aria-label={`Live terminal of ${agentName}`}
    class="h-96 overflow-hidden rounded-md bg-gray-900 p-2"
  ></div>
</section>
