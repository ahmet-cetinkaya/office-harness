// SPDX-License-Identifier: AGPL-3.0-or-later

import { visibleText } from "../../agent";

/** The largest content or diff the office sends, in bytes (contract). */
export const SIZE_LIMIT_BYTES = 524_288;
/** The path of the root, which only `agent.files.list` accepts. */
export const ROOT_PATH = "";

export const EMPTY_FOLDER_TEXT = "This folder is empty.";
export const EMPTY_FILE_TEXT = "This file is empty.";
export const NO_CHANGES_TEXT = "No change since the last commit.";
export const NO_COMMITS_TEXT = "No commit yet.";
export const NOT_REPOSITORY_TEXT = "This folder is not in a git repository.";
export const SECRET_FILE_TEXT = "This file may hold a secret.";
export const SECRET_DIFF_TEXT = "This change may hold a secret.";
export const BINARY_FILE_TEXT = "Binary file, not shown";
export const NO_DIFF_TEXT = "This file has no change here.";
export const LINK_OUTSIDE_TEXT =
  "This link points outside the agent's working folder. Nothing of its target is shown.";
export const LINK_BROKEN_TEXT =
  "The target of this link is missing or forms a loop. Nothing is shown.";
export const NOT_A_FILE_TEXT =
  "This entry is not a regular file, so it is not opened.";
export const NAME_NOT_UTF8_TEXT =
  "The office cannot open a name that is not valid UTF-8.";
export const NO_NEWLINE_MARKER = "\\ No newline at end of file";

const REPLACEMENT_CHARACTER = "�";
const LINE_FEED = "\n";
const TAB = "\t";
const TRAILING_CARRIAGE_RETURN = /\r$/;
const HUNK_HEADER = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/;
const KIBIBYTE = 1024;

export type EntryKind = "folder" | "file" | "link" | "other";
export type LinkTarget = "folder" | "file" | "other" | "outside" | "broken";

export interface FileEntry {
  name: string;
  kind: EntryKind;
  size: number | null;
  secret: boolean;
  target: LinkTarget | null;
}

export interface FileList {
  root: string;
  path: string;
  entries: FileEntry[];
  more: number;
  read_at: number;
}

export type ReadState = "secret" | "too_large" | "binary" | "text";

export interface FileRead {
  path: string;
  state: ReadState;
  size: number;
  content_base64: string | null;
  read_at: number;
}

export type ChangeKind =
  | "modified"
  | "added"
  | "deleted"
  | "renamed"
  | "type_changed"
  | "untracked"
  | "conflicted"
  | "submodule";

export interface ChangedFile {
  path: string;
  kind: ChangeKind;
  old_path: string | null;
  secret: boolean;
}

export interface ChangeSet {
  repository: boolean;
  files: ChangedFile[];
  more: number;
  read_at: number;
}

export type DiffState = "secret" | "too_large" | "binary" | "none" | "diff";

export interface FileDiff {
  path: string;
  state: DiffState;
  size: number | null;
  content_base64: string | null;
}

export interface Commit {
  id: string;
  short_id: string;
  subject: string;
  author: string;
  time: number;
}

export interface HistoryPage {
  repository: boolean;
  commits: Commit[];
  more: boolean;
}

export interface CommitDetail {
  id: string;
  parents: string[];
  author: string;
  email: string;
  time: number;
  message: string;
  files: ChangedFile[];
  more: number;
}

/** The change in one file that a diff asks for, in the working folder or in a commit. */
export interface DiffTarget {
  path: string;
  oldPath: string | null;
  commit: string | null;
}

export type DiffLineKind = "hunk" | "added" | "removed" | "context" | "note";

export interface DiffLine {
  kind: DiffLineKind;
  oldNumber: number | null;
  newNumber: number | null;
  text: string;
}

/** The bytes as UTF-8, each invalid sequence replaced by U+FFFD (A9). */
export function decodeContent(contentBase64: string): string {
  const binary = atob(contentBase64);
  const bytes = Uint8Array.from(binary, (character) => character.charCodeAt(0));
  return new TextDecoder("utf-8", { fatal: false }).decode(bytes);
}

/**
 * One line in visible form (FR-009, D13): a tab stays, so code keeps its
 * layout; every other control or bidirectional format character becomes a
 * visible symbol; a carriage return at the end belongs to the line break.
 */
export function visibleLine(line: string): string {
  return line
    .replace(TRAILING_CARRIAGE_RETURN, "")
    .split(TAB)
    .map(visibleText)
    .join(TAB);
}

function splitLines(text: string): string[] {
  if (text === "") return [];
  const lines = text.split(LINE_FEED);
  if (lines[lines.length - 1] === "") lines.pop();
  return lines;
}

/** The lines of a text file in visible form; an empty file has none. */
export function contentLines(text: string): string[] {
  return splitLines(text).map(visibleLine);
}

/** Whether a name holds a byte sequence that was not valid UTF-8 (contract, Paths). */
export function hasInvalidName(name: string): boolean {
  return name.includes(REPLACEMENT_CHARACTER);
}

/** The lines of a unified diff with their old and new numbers. */
export function parseDiff(text: string): DiffLine[] {
  const lines: DiffLine[] = [];
  let oldNumber = 0;
  let newNumber = 0;
  let isInsideHunk = false;
  for (const raw of splitLines(text)) {
    const header = HUNK_HEADER.exec(raw);
    if (header) {
      oldNumber = Number(header[1]);
      newNumber = Number(header[2]);
      isInsideHunk = true;
      lines.push({
        kind: "hunk",
        oldNumber: null,
        newNumber: null,
        text: visibleLine(raw),
      });
      continue;
    }
    if (!isInsideHunk) continue;
    const sign = raw.charAt(0);
    const body = visibleLine(raw.slice(1));
    if (sign === "+") {
      lines.push({ kind: "added", oldNumber: null, newNumber, text: body });
      newNumber += 1;
    } else if (sign === "-") {
      lines.push({ kind: "removed", oldNumber, newNumber: null, text: body });
      oldNumber += 1;
    } else if (sign === "\\") {
      lines.push({
        kind: "note",
        oldNumber: null,
        newNumber: null,
        text: visibleLine(raw),
      });
    } else {
      lines.push({ kind: "context", oldNumber, newNumber, text: body });
      oldNumber += 1;
      newNumber += 1;
    }
  }
  return lines;
}

/** A byte count with thousands separators, for example "1,048,576 bytes". */
export function formatByteCount(size: number): string {
  return `${size.toLocaleString("en-US")} bytes`;
}

const LIMIT_TEXT = `${SIZE_LIMIT_BYTES / KIBIBYTE} KiB`;

/** The "too large" text: the size when known, and the limit (FR-002, FR-005). */
export function tooLargeText(subject: string, size: number | null): string {
  const sizeText =
    size === null ? "is over the limit" : `is ${formatByteCount(size)}`;
  return `${subject} ${sizeText}. Content over ${LIMIT_TEXT} is not shown.`;
}

/** The time of a load, for the header. */
export function formatLoadedAt(time: number): string {
  return new Date(time).toLocaleTimeString();
}

/** The time of a commit, with its date. */
export function formatCommitTime(time: number): string {
  return new Date(time).toLocaleString();
}

/** The path of a child in a folder, `/` between parts. */
export function childPath(folder: string, name: string): string {
  return folder === ROOT_PATH ? name : `${folder}/${name}`;
}

/** A folder, or a link to a folder inside the root: it opens as a folder. */
export function isFolderEntry(entry: FileEntry): boolean {
  return (
    entry.kind === "folder" ||
    (entry.kind === "link" && entry.target === "folder")
  );
}

const CHANGE_KIND_TEXT: Record<ChangeKind, string> = {
  modified: "modified",
  added: "added",
  deleted: "deleted",
  renamed: "renamed",
  type_changed: "type changed",
  untracked: "untracked",
  conflicted: "conflicted",
  submodule: "submodule",
};

/** The kind of a change as words, for a row (the kind is never color alone). */
export function changeKindText(kind: ChangeKind): string {
  return CHANGE_KIND_TEXT[kind];
}

/** A block of text, such as a commit message, in visible form. */
export function visibleBlock(text: string): string {
  return contentLines(text).join(LINE_FEED);
}
