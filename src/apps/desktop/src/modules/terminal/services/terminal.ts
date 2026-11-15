// SPDX-License-Identifier: AGPL-3.0-or-later

const KIND_BYTE_SNAPSHOT = 0x01;
const KIND_BYTE_OUTPUT = 0x02;
const KIND_BYTE_DROPPED = 0x03;
const KIND_BYTES = 1;
const AGENT_ID_BYTES = 16;
const HEADER_BYTES = KIND_BYTES + AGENT_ID_BYTES;
const UUID_GROUP_ENDS = [4, 6, 8, 10];
const BASE64_CHUNK_BYTES = 0x8000;
const ESCAPE = "\u001b";

export type FrameKind = "snapshot" | "output" | "dropped";

export interface TerminalFrame {
  kind: FrameKind;
  agentId: string;
  payload: Uint8Array;
}

const KINDS = new Map<number, FrameKind>([
  [KIND_BYTE_SNAPSHOT, "snapshot"],
  [KIND_BYTE_OUTPUT, "output"],
  [KIND_BYTE_DROPPED, "dropped"],
]);

function uuidFromBytes(bytes: Uint8Array): string {
  const hex = Array.from(bytes, (byte) =>
    byte.toString(16).padStart(2, "0"),
  ).join("");
  const groups: string[] = [];
  let start = 0;
  for (const end of [...UUID_GROUP_ENDS.map((byte) => byte * 2), hex.length]) {
    groups.push(hex.slice(start, end));
    start = end;
  }
  return groups.join("-");
}

/** Reads one binary frame; null for a frame this client does not know. */
export function parseFrame(bytes: Uint8Array): TerminalFrame | null {
  const kind = KINDS.get(bytes[0]);
  if (bytes.length < HEADER_BYTES || kind === undefined) return null;
  return {
    kind,
    agentId: uuidFromBytes(bytes.subarray(KIND_BYTES, HEADER_BYTES)),
    payload: bytes.subarray(HEADER_BYTES),
  };
}

// The terminal answers these queries itself, so the server also answers them
// and a CLI would otherwise get two replies (contract: "Terminal queries").
// Cursor position: ESC [ row ; col R, or ESC [ ? row ; col R.
const CURSOR_POSITION_REPLY = new RegExp(`${ESCAPE}\\[\\??\\d+;\\d+R`, "g");
// Device attributes: ESC [ ? ... c (primary) and ESC [ > ... c (secondary).
const DEVICE_ATTRIBUTES_REPLY = new RegExp(
  `${ESCAPE}\\[[?>]\\d*(?:;\\d+)*c`,
  "g",
);

/**
 * Removes xterm.js's automatic replies to cursor-position and
 * device-attribute queries from `chunk`. The caller must not pass keystrokes
 * through it: Ctrl+F3 sends the same bytes as a cursor-position reply.
 */
export function dropQueryReplies(chunk: string): string {
  return chunk
    .replace(CURSOR_POSITION_REPLY, "")
    .replace(DEVICE_ATTRIBUTES_REPLY, "");
}

function toBase64(bytes: Uint8Array): string {
  let binary = "";
  for (let start = 0; start < bytes.length; start += BASE64_CHUNK_BYTES) {
    binary += String.fromCharCode(
      ...bytes.subarray(start, start + BASE64_CHUNK_BYTES),
    );
  }
  return btoa(binary);
}

/** The `data` field of `terminal.input` for text that xterm.js emits. */
export function encodeInput(text: string): string {
  return toBase64(new TextEncoder().encode(text));
}

/** The same for xterm.js's binary input (mouse reports): one byte per char. */
export function encodeBinaryInput(text: string): string {
  return toBase64(
    Uint8Array.from(text, (character) => character.charCodeAt(0)),
  );
}
