// SPDX-License-Identifier: AGPL-3.0-or-later

import type {
  ChangedFile,
  Commit,
  FileEntry,
} from "../../src/modules/files/models/file-view";

/** The agent whose folder is not in a git repository. */
export const AGENT_NOT_REPOSITORY = "agent-plain";
/** The agent whose repository has no change and no commit. */
export const AGENT_CLEAN = "agent-clean";
/** The number of commits of the seeded history: more than two pages. */
export const SEEDED_COMMIT_COUNT = 120;
/** The script flag that a hostile text would set if anything ran it. */
export const SCRIPT_FLAG = "__fileViewScriptRan";
export const HOSTILE_FILE_IMAGE = "/hostile-file-image.png";
export const HOSTILE_COMMIT_IMAGE = "/hostile-commit-image.png";

/** What the mock answers for one file or one diff. */
export interface FixtureContent {
  state: string;
  size: number | null;
  content_base64: string | null;
  /** A secret-looking file answers `secret` until a request reveals it. */
  secret?: boolean;
}

export interface FileFixture {
  notRepositoryAgentId: string;
  cleanAgentId: string;
  root: string;
  readAt: number;
  folders: Record<string, FileEntry[]>;
  reads: Record<string, FixtureContent>;
  changes: ChangedFile[];
  diffs: Record<string, FixtureContent>;
  commits: Commit[];
  commitMessages: Record<string, string>;
}

const ROOT_READ_AT = 1_700_000_100_000;
const COMMIT_TIME = 1_700_000_000_000;
const MINUTE = 60_000;
const HEX_ID_LENGTH = 40;
const TOO_LARGE_SIZE = 1_048_576;

const HOSTILE_LINES = [
  `<script>window.${SCRIPT_FLAG} = true</script>`,
  `<img src="${HOSTILE_FILE_IMAGE}" onerror="window.${SCRIPT_FLAG} = true">`,
  "escape: \u001b[31mred",
  "override: ‮reversed",
  "tab:\tkept",
];

const MAIN_DIFF = [
  "@@ -1,3 +1,4 @@",
  " first line",
  "-second line",
  "+second line changed",
  "+inserted line",
  " third line",
  "@@ -9,2 +10,2 @@",
  " context nine",
  "-last old",
  "\\ No newline at end of file",
  "+last new",
  "\\ No newline at end of file",
  "",
].join("\n");

function encode(text: string): string {
  return Buffer.from(text, "utf8").toString("base64");
}

function text(content: string, secret = false): FixtureContent {
  return {
    state: "text",
    size: Buffer.byteLength(content),
    content_base64: encode(content),
    secret,
  };
}

function withoutContent(state: string, size: number | null): FixtureContent {
  return { state, size, content_base64: null };
}

function file(name: string, size: number, secret = false): FileEntry {
  return { name, kind: "file", size, secret, target: null };
}

function folder(name: string): FileEntry {
  return { name, kind: "folder", size: null, secret: false, target: null };
}

function link(name: string, target: FileEntry["target"]): FileEntry {
  return { name, kind: "link", size: null, secret: false, target };
}

function addedDiff(lines: string[]): string {
  const body = lines.map((line) => `+${line}`).join("\n");
  return `@@ -0,0 +1,${lines.length} @@\n${body}\n`;
}

function commitId(index: number): string {
  return index.toString(16).padStart(HEX_ID_LENGTH, "0");
}

function seededCommits(): Commit[] {
  return Array.from({ length: SEEDED_COMMIT_COUNT }, (_, index) => ({
    id: commitId(index + 1),
    short_id: commitId(index + 1).slice(-7),
    subject:
      index === 0
        ? `Fix <script>window.${SCRIPT_FLAG} = true</script> \u001b[31m ‮`
        : `Change number ${SEEDED_COMMIT_COUNT - index}`,
    author: "Ada Lovelace",
    time: COMMIT_TIME - index * MINUTE,
  }));
}

/**
 * The folder that the mock lists for the file view: folders, a text file, a
 * secret-looking file, a file that is too large, a binary file, links, an
 * empty folder, hostile texts, changes with a renamed secret file, and a
 * history of more than two pages. Plain data, so it can cross into the page.
 */
export function buildFileFixture(): FileFixture {
  const hostile = HOSTILE_LINES.join("\n") + "\n";
  const mainText = "first line\nsecond line\nthird line\n";
  const commits = seededCommits();
  return {
    notRepositoryAgentId: AGENT_NOT_REPOSITORY,
    cleanAgentId: AGENT_CLEAN,
    root: "/work/ada",
    readAt: ROOT_READ_AT,
    folders: {
      "": [
        folder("empty"),
        folder("src"),
        file(".env", 18, true),
        file("bad-�name.txt", 4),
        file("big.log", TOO_LARGE_SIZE),
        link("broken-link", "broken"),
        file("hostile.txt", Buffer.byteLength(hostile)),
        file("image.bin", 12),
        file("notes.txt", 12),
        link("outside-link", "outside"),
      ],
      src: [file("main.txt", Buffer.byteLength(mainText))],
      empty: [],
    },
    reads: {
      ".env": text("API_TOKEN=abc\n", true),
      "big.log": withoutContent("too_large", TOO_LARGE_SIZE),
      "hostile.txt": text(hostile),
      "image.bin": withoutContent("binary", 12),
      "notes.txt": text("plain notes\n"),
      "src/main.txt": text(mainText),
    },
    changes: [
      { path: "gone.txt", kind: "deleted", old_path: null, secret: false },
      { path: "hostile.txt", kind: "added", old_path: null, secret: false },
      { path: "huge.txt", kind: "modified", old_path: null, secret: false },
      { path: "image.bin", kind: "modified", old_path: null, secret: false },
      { path: "new.txt", kind: "untracked", old_path: null, secret: false },
      { path: "notes.txt", kind: "renamed", old_path: ".env", secret: true },
      { path: "quiet.txt", kind: "modified", old_path: null, secret: false },
      { path: "src/main.txt", kind: "modified", old_path: null, secret: false },
    ],
    diffs: {
      "gone.txt": text("@@ -1 +0,0 @@\n-removed content\n"),
      "hostile.txt": text(addedDiff(HOSTILE_LINES)),
      "huge.txt": withoutContent("too_large", null),
      "image.bin": withoutContent("binary", null),
      "new.txt": text(addedDiff(["brand new line"])),
      "notes.txt": text("@@ -1 +1 @@\n-API_TOKEN=abc\n+API_TOKEN=abc\n", true),
      "quiet.txt": withoutContent("none", null),
      "src/main.txt": text(MAIN_DIFF),
    },
    commits,
    commitMessages: {
      [commits[0].id]: [
        commits[0].subject,
        "",
        `<img src="${HOSTILE_COMMIT_IMAGE}">`,
        "second paragraph",
      ].join("\n"),
    },
  };
}
