// SPDX-License-Identifier: AGPL-3.0-or-later

// Builds office-harnessd for the current machine and copies it to the name
// Tauri expects for a sidecar: binaries/office-harnessd-<target-triple>[.exe].

import { cpSync, mkdirSync } from "node:fs";
import { join } from "node:path";

const repositoryRoot = join(import.meta.dir, "..", "..", "..", "..");
const sidecarName = "office-harnessd";
const buildProfile = process.argv.includes("--release") ? "release" : "debug";

function runOrExit(command: string[]): string {
  const outcome = Bun.spawnSync(command, {
    cwd: repositoryRoot,
    stderr: "inherit",
  });
  if (outcome.exitCode !== 0) {
    console.error(
      `Command failed with exit code ${outcome.exitCode}: ${command.join(" ")}`,
    );
    process.exit(outcome.exitCode ?? 1);
  }
  return outcome.stdout.toString();
}

function hostTargetTriple(): string {
  const hostLine = runOrExit(["rustc", "-vV"])
    .split("\n")
    .find((line) => line.startsWith("host: "));
  if (!hostLine) {
    throw new Error("rustc -vV did not report a host target triple.");
  }
  return hostLine.slice("host: ".length).trim();
}

const targetTriple = hostTargetTriple();
const extension = targetTriple.includes("windows") ? ".exe" : "";
const buildCommand = ["cargo", "build", "--package", sidecarName];
if (buildProfile === "release") {
  buildCommand.push("--release");
}
runOrExit(buildCommand);

const builtBinary = join(
  repositoryRoot,
  "target",
  buildProfile,
  `${sidecarName}${extension}`,
);
const binariesFolder = join(
  repositoryRoot,
  "src",
  "apps",
  "desktop",
  "src-tauri",
  "binaries",
);
mkdirSync(binariesFolder, { recursive: true });
cpSync(
  builtBinary,
  join(binariesFolder, `${sidecarName}-${targetTriple}${extension}`),
);
