<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<script lang="ts">
  import { FitAddon } from "@xterm/addon-fit";
  import { WebglAddon } from "@xterm/addon-webgl";
  import { Terminal } from "@xterm/xterm";
  import "@xterm/xterm/css/xterm.css";
  import { onMount } from "svelte";
  import FailureMessage from "$shared/ui/failure-message.svelte";
  import { toCoreRequestError, type CoreClient } from "../../../core/api";
  import {
    matchesShortcut,
    pressedAppShortcut,
    sharedFocusShortcut,
    type FocusShortcut,
  } from "../../../core/shortcuts";
  import { createInputQueue } from "../services/input-queue";
  import {
    binaryBytes,
    filterEmittedData,
    textBytes,
    type TerminalFrame,
  } from "../services/terminal";
  import { terminalFeedFor } from "../services/terminal-feed";

  interface Props {
    client: CoreClient;
    agentId: string;
    agentName: string;
    /** A stopped agent shows the output of its last session and takes no input. */
    isRunning: boolean;
    /**
     * Runs on the focus shortcut instead of the default return to this
     * view's heading; the scene uses it to hand focus back to the desk
     * (0005 D5). The heading keeps the default when left out.
     */
    onFocusLeave?: () => void;
  }

  let { client, agentId, agentName, isRunning, onFocusLeave }: Props = $props();

  const RESIZE_DEBOUNCE_MS = 100;
  const SCROLLBACK_LINES = 10_000;
  // Light grey on near black is 15:1, well above the 4.5:1 that text needs.
  const FUNCTION_KEY = /^F\d{1,2}$/;
  const THEME = { background: "#111827", foreground: "#f3f4f6" };

  let host: HTMLDivElement;
  let heading: HTMLHeadingElement;
  let focusShortcut = $state<FocusShortcut | null>(null);
  let hasDroppedOutput = $state(false);
  let problem = $state<string | null>(null);
  let terminalInput = $state<HTMLTextAreaElement | null>(null);

  $effect(() => {
    terminalInput?.setAttribute("aria-label", `Terminal of ${agentName}`);
  });

  onMount(() => {
    const terminal = new Terminal({
      theme: THEME,
      scrollback: SCROLLBACK_LINES,
      // Puts the screen text in the page for screen readers.
      screenReaderMode: true,
      disableStdin: !isRunning,
    });
    const fit = new FitAddon();
    let isDisposed = false;
    let isTypingKey = false;
    // The server refuses a resize before the attach, which sends the size.
    let isAttached = false;
    // Output that comes before the snapshot belongs to the screen it replaces.
    let hasSnapshot = false;
    let stopListening = () => {};
    let resizeTimer: ReturnType<typeof setTimeout> | undefined;

    function report(request: Promise<unknown>): void {
      request.catch((failure: unknown) => {
        problem = toCoreRequestError(failure).message;
      });
    }

    const input = createInputQueue(
      (encoded) =>
        client.sendRequest("terminal.input", {
          agent_id: agentId,
          data: encoded,
        }),
      (failure) => {
        problem = toCoreRequestError(failure).message;
      },
    );

    function isEditable(element: Element | null): boolean {
      return (
        element instanceof HTMLElement &&
        (element.matches("input, textarea, select") ||
          element.isContentEditable)
      );
    }

    function requestResize(cols: number, rows: number): void {
      report(
        client.sendRequest("terminal.resize", {
          agent_id: agentId,
          cols,
          rows,
        }),
      );
    }

    function showFrame(frame: TerminalFrame): void {
      if (frame.kind === "dropped") {
        hasDroppedOutput = true;
        return;
      }
      if (frame.kind === "snapshot") {
        terminal.reset();
        hasSnapshot = true;
      } else if (!hasSnapshot) {
        return;
      }
      terminal.write(frame.payload);
    }

    // The keys that do not reach the agent (0003, 0001/FR-017 modified):
    // the focus shortcut, and the app shortcut table the panel listens on.
    function handleKey(event: KeyboardEvent): boolean {
      if (
        focusShortcut !== null &&
        matchesShortcut(focusShortcut.shortcut, event)
      ) {
        if (event.type === "keydown") {
          // The webview would otherwise run its own action for the key,
          // such as the developer tools on F12.
          event.preventDefault();
          if (onFocusLeave !== undefined) onFocusLeave();
          else heading.focus();
        }
        return false;
      }
      if (pressedAppShortcut(event) !== null) {
        if (event.type === "keydown") {
          // The webview would otherwise run its own action for an F-key,
          // and xterm.js would send the key's sequence to the CLI.
          event.preventDefault();
        }
        // Returning false keeps the sequence out of the terminal and lets
        // the keydown bubble to the panel's app-level handler.
        return false;
      }
      // A stopped terminal takes no input, so Tab must reach the browser and
      // move focus on. xterm.js would otherwise cancel it.
      if (!isRunning && event.key === "Tab") return false;
      // The webview would otherwise move focus on Tab and open its own
      // function-key actions. xterm.js still sends the key's sequence.
      if (
        isRunning &&
        event.type === "keydown" &&
        (event.key === "Tab" || FUNCTION_KEY.test(event.key))
      ) {
        event.preventDefault();
      }
      // xterm.js emits a keystroke's data inside this call. Only data that
      // comes later is an automatic reply, which dropQueryReplies removes.
      isTypingKey = true;
      queueMicrotask(() => (isTypingKey = false));
      return true;
    }

    function loadRenderer(): void {
      // Headless automation gets xterm's DOM renderer, which draws text that
      // tests and screenshots can see; WebGL draws an empty canvas there.
      if (navigator.webdriver) return;
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
      focusShortcut = await sharedFocusShortcut(client);
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
      const { cols, rows } = terminal;
      await client.sendRequest("terminal.attach", {
        agent_id: agentId,
        cols,
        rows,
      });
      isAttached = true;
      // A resize during the attach had nowhere to go, so send the final size.
      if (terminal.cols !== cols || terminal.rows !== rows) {
        requestResize(terminal.cols, terminal.rows);
      }
      if (isRunning && !isEditable(document.activeElement)) terminal.focus();
    }

    terminal.loadAddon(fit);
    terminal.open(host);
    terminalInput = terminal.textarea ?? null;
    loadRenderer();
    terminal.attachCustomKeyEventHandler(handleKey);
    terminal.onData((text) => {
      const forwarded = filterEmittedData(text, isTypingKey);
      if (forwarded !== "") input.push(textBytes(forwarded));
    });
    terminal.onBinary((text) => input.push(binaryBytes(text)));
    terminal.onResize(({ cols, rows }) => {
      if (isAttached) requestResize(cols, rows);
    });
    const observer = new ResizeObserver(scheduleFit);
    observer.observe(host);
    report(attach());

    return () => {
      isDisposed = true;
      terminalInput = null;
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

<section aria-label="Terminal" class="flex flex-col gap-2">
  <h3
    id="terminal-heading"
    bind:this={heading}
    tabindex="-1"
    aria-label="Terminal, {agentName}"
    class="text-lg font-semibold text-neutral-900 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-700"
  >
    Terminal
  </h3>
  <p class="text-sm text-neutral-900">
    {#if !isRunning}
      The agent is stopped. The terminal shows the output of its last session.
    {:else if focusShortcut !== null}
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
