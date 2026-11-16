// SPDX-License-Identifier: AGPL-3.0-or-later

import type { EnvironmentEntry } from "../models/agent";

const ENV_SEPARATOR = "=";

export type EnvParse =
  { ok: true; entries: EnvironmentEntry[] } | { ok: false; message: string };

function nonEmptyLines(text: string): string[] {
  return text
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line !== "");
}

/** One flag for each line. Each line is one argument, so a value is its own line. */
export function parseFlags(text: string): string[] {
  return nonEmptyLines(text);
}

export function formatFlags(flags: string[]): string {
  return flags.join("\n");
}

/** One NAME=value for each line. The value is what follows the first `=`. */
export function parseEnvironment(text: string): EnvParse {
  const entries: EnvironmentEntry[] = [];
  for (const line of nonEmptyLines(text)) {
    const separatorAt = line.indexOf(ENV_SEPARATOR);
    if (separatorAt < 0) {
      return { ok: false, message: `"${line}" needs the form NAME=value.` };
    }
    const name = line.slice(0, separatorAt).trim();
    if (name === "") {
      return { ok: false, message: `"${line}" has no name before the =.` };
    }
    entries.push({ name, value: line.slice(separatorAt + 1) });
  }
  return { ok: true, entries };
}

export function formatEnvironment(entries: EnvironmentEntry[]): string {
  return entries.map((entry) => `${entry.name}=${entry.value}`).join("\n");
}

/** An empty prompt means no prompt. */
export function parseStartupPrompt(text: string): string | null {
  return text.trim() === "" ? null : text;
}
