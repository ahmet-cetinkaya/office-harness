// SPDX-License-Identifier: AGPL-3.0-or-later

import { toCoreRequestError } from "../../../core/api";
import {
  contentLines,
  decodeContent,
  hasInvalidName,
  isFolderEntry,
  LINK_BROKEN_TEXT,
  LINK_OUTSIDE_TEXT,
  NAME_NOT_UTF8_TEXT,
  NOT_A_FILE_TEXT,
  parseDiff,
  ROOT_PATH,
  type ChangedFile,
  type Commit,
  type CommitDetail,
  type DiffLine,
  type DiffTarget,
  type FileDiff,
  type FileEntry,
  type FileRead,
} from "../models/file-view";
import type { FileViewService } from "../services/file-view-service";

const REQUEST_FAILED = "The office did not answer.";

export type FileViewTab = "files" | "changes" | "history";
export type Phase = "loading" | "ready" | "empty" | "failed";

export interface FolderState {
  phase: Phase;
  entries: FileEntry[];
  more: number;
  failure: string | null;
}

/** The file on show. `notice` is an entry that the view never asks the office for. */
export interface ViewedFile {
  path: string;
  phase: "loading" | "ready" | "failed" | "notice";
  message: string | null;
  read: FileRead | null;
  lines: string[];
  revealed: boolean;
}

/** The diff on show, in the Changes part or in an opened commit. */
export interface ViewedDiff {
  target: DiffTarget;
  phase: "loading" | "ready" | "failed";
  failure: string | null;
  diff: FileDiff | null;
  lines: DiffLine[];
  revealed: boolean;
}

export interface OpenedCommit {
  id: string;
  phase: "loading" | "ready" | "failed";
  failure: string | null;
  detail: CommitDetail | null;
  selectedPath: string | null;
}

interface FilesPart {
  folders: Record<string, FolderState>;
  expanded: Record<string, boolean>;
  selectedPath: string | null;
  viewed: ViewedFile | null;
  readAt: number | null;
}

interface ChangesPart {
  phase: Phase;
  failure: string | null;
  repository: boolean;
  files: ChangedFile[];
  more: number;
  selectedPath: string | null;
  viewedDiff: ViewedDiff | null;
  readAt: number | null;
}

interface HistoryPart {
  phase: Phase;
  failure: string | null;
  repository: boolean;
  commits: Commit[];
  hasMore: boolean;
  isLoadingMore: boolean;
  moreFailure: string | null;
  opened: OpenedCommit | null;
  viewedDiff: ViewedDiff | null;
  readAt: number | null;
}

function describe(rejection: unknown): string {
  return toCoreRequestError(rejection).message || REQUEST_FAILED;
}

function loadingFolder(): FolderState {
  return { phase: "loading", entries: [], more: 0, failure: null };
}

function emptyFilesPart(): FilesPart {
  return {
    folders: {},
    expanded: {},
    selectedPath: null,
    viewed: null,
    readAt: null,
  };
}

function emptyChangesPart(): ChangesPart {
  return {
    phase: "loading",
    failure: null,
    repository: true,
    files: [],
    more: 0,
    selectedPath: null,
    viewedDiff: null,
    readAt: null,
  };
}

function emptyHistoryPart(): HistoryPart {
  return {
    phase: "loading",
    failure: null,
    repository: true,
    commits: [],
    hasMore: false,
    isLoadingMore: false,
    moreFailure: null,
    opened: null,
    viewedDiff: null,
    readAt: null,
  };
}

function noticeFile(path: string, message: string): ViewedFile {
  return {
    path,
    phase: "notice",
    message,
    read: null,
    lines: [],
    revealed: false,
  };
}

/** What the office is not asked for: a link that leaves the folder, or a special file. */
function noticeFor(entry: FileEntry): string | null {
  if (hasInvalidName(entry.name)) return NAME_NOT_UTF8_TEXT;
  if (entry.kind === "other") return NOT_A_FILE_TEXT;
  if (entry.kind !== "link") return null;
  if (entry.target === "outside") return LINK_OUTSIDE_TEXT;
  if (entry.target === "broken") return LINK_BROKEN_TEXT;
  return entry.target === "other" ? NOT_A_FILE_TEXT : null;
}

/**
 * What the file view of one agent shows and loads (D10). Every part loads when
 * it is first shown. A reveal lives in the viewed file or diff only, so every
 * open, tab switch, and refresh drops it (FR-004). An answer that comes after
 * the view moved on is dropped by the counters below.
 */
export function createFileViewState(service: FileViewService, agentId: string) {
  const view = $state({
    tab: "files" as FileViewTab,
    root: null as string | null,
    files: emptyFilesPart(),
    changes: emptyChangesPart(),
    history: emptyHistoryPart(),
  });
  let loadGeneration = 0;
  let fileRequest = 0;
  let diffRequest = 0;
  let commitRequest = 0;
  const startedTabs: Record<FileViewTab, boolean> = {
    files: false,
    changes: false,
    history: false,
  };

  async function loadFolder(path: string): Promise<void> {
    const generation = loadGeneration;
    view.files.folders[path] = loadingFolder();
    try {
      const list = await service.list(agentId, path);
      if (generation !== loadGeneration) return;
      view.files.folders[path] = {
        phase: list.entries.length === 0 ? "empty" : "ready",
        entries: list.entries,
        more: list.more,
        failure: null,
      };
      if (path === ROOT_PATH) {
        view.root = list.root;
        view.files.readAt = list.read_at;
      }
    } catch (rejection) {
      if (generation !== loadGeneration) return;
      view.files.folders[path] = {
        phase: "failed",
        entries: [],
        more: 0,
        failure: describe(rejection),
      };
    }
  }

  async function loadChanges(): Promise<void> {
    const generation = loadGeneration;
    view.changes = emptyChangesPart();
    try {
      const set = await service.changes(agentId);
      if (generation !== loadGeneration) return;
      view.changes = {
        ...emptyChangesPart(),
        phase: set.files.length === 0 ? "empty" : "ready",
        repository: set.repository,
        files: set.files,
        more: set.more,
        readAt: set.read_at,
      };
    } catch (rejection) {
      if (generation !== loadGeneration) return;
      view.changes.phase = "failed";
      view.changes.failure = describe(rejection);
    }
  }

  async function loadHistory(): Promise<void> {
    const generation = loadGeneration;
    view.history = emptyHistoryPart();
    try {
      const page = await service.history(agentId, 0);
      if (generation !== loadGeneration) return;
      view.history = {
        ...emptyHistoryPart(),
        phase: page.commits.length === 0 ? "empty" : "ready",
        repository: page.repository,
        commits: page.commits,
        hasMore: page.more,
        readAt: Date.now(),
      };
    } catch (rejection) {
      if (generation !== loadGeneration) return;
      view.history.phase = "failed";
      view.history.failure = describe(rejection);
    }
  }

  async function showMoreCommits(): Promise<void> {
    if (view.history.isLoadingMore) return;
    const generation = loadGeneration;
    view.history.isLoadingMore = true;
    view.history.moreFailure = null;
    try {
      const page = await service.history(agentId, view.history.commits.length);
      if (generation !== loadGeneration) return;
      view.history.commits.push(...page.commits);
      view.history.hasMore = page.more;
    } catch (rejection) {
      if (generation !== loadGeneration) return;
      view.history.moreFailure = describe(rejection);
    } finally {
      if (generation === loadGeneration) view.history.isLoadingMore = false;
    }
  }

  function startTab(tab: FileViewTab): void {
    if (startedTabs[tab]) return;
    startedTabs[tab] = true;
    if (tab === "files") void loadFolder(ROOT_PATH);
    else if (tab === "changes") void loadChanges();
    else void loadHistory();
  }

  /** Drops what the user was reading, and with it every reveal. */
  function clearViewed(): void {
    fileRequest += 1;
    diffRequest += 1;
    view.files.selectedPath = null;
    view.files.viewed = null;
    view.changes.selectedPath = null;
    view.changes.viewedDiff = null;
    view.history.viewedDiff = null;
    if (view.history.opened !== null) view.history.opened.selectedPath = null;
  }

  function showTab(tab: FileViewTab): void {
    if (tab !== view.tab) clearViewed();
    view.tab = tab;
    startTab(tab);
  }

  function refresh(): void {
    const expanded = Object.keys(view.files.expanded).filter(
      (path) => view.files.expanded[path],
    );
    loadGeneration += 1;
    commitRequest += 1;
    clearViewed();
    for (const tab of Object.keys(startedTabs) as FileViewTab[]) {
      startedTabs[tab] = false;
    }
    view.files = emptyFilesPart();
    view.changes = emptyChangesPart();
    view.history = emptyHistoryPart();
    startTab(view.tab);
    if (view.tab !== "files") return;
    for (const path of expanded) {
      view.files.expanded[path] = true;
      void loadFolder(path);
    }
  }

  async function requestFile(path: string, reveal: boolean): Promise<void> {
    const request = ++fileRequest;
    view.files.viewed = {
      path,
      phase: "loading",
      message: null,
      read: null,
      lines: [],
      revealed: reveal,
    };
    try {
      const read = await service.read(agentId, path, reveal);
      if (request !== fileRequest) return;
      view.files.viewed = {
        path,
        phase: "ready",
        message: null,
        read,
        lines:
          read.content_base64 === null
            ? []
            : contentLines(decodeContent(read.content_base64)),
        revealed: reveal,
      };
    } catch (rejection) {
      if (request !== fileRequest) return;
      view.files.viewed = {
        path,
        phase: "failed",
        message: describe(rejection),
        read: null,
        lines: [],
        revealed: reveal,
      };
    }
  }

  function toggleFolder(path: string): void {
    setExpanded(path, !view.files.expanded[path]);
  }

  function setExpanded(path: string, isExpanded: boolean): void {
    view.files.expanded[path] = isExpanded;
    if (isExpanded && view.files.folders[path] === undefined) {
      void loadFolder(path);
    }
  }

  /** Opens a tree entry: a folder shows its entries, anything else shows as a file. */
  function openEntry(path: string, entry: FileEntry): void {
    fileRequest += 1;
    view.files.selectedPath = path;
    view.files.viewed = null;
    if (isFolderEntry(entry)) {
      toggleFolder(path);
      return;
    }
    const notice = noticeFor(entry);
    if (notice !== null) view.files.viewed = noticeFile(path, notice);
    else void requestFile(path, false);
  }

  async function requestDiff(
    target: DiffTarget,
    reveal: boolean,
  ): Promise<void> {
    const request = ++diffRequest;
    const part = view.tab === "history" ? view.history : view.changes;
    part.viewedDiff = {
      target,
      phase: "loading",
      failure: null,
      diff: null,
      lines: [],
      revealed: reveal,
    };
    try {
      const diff = await service.diff(agentId, target, reveal);
      if (request !== diffRequest) return;
      part.viewedDiff = {
        target,
        phase: "ready",
        failure: null,
        diff,
        lines:
          diff.content_base64 === null
            ? []
            : parseDiff(decodeContent(diff.content_base64)),
        revealed: reveal,
      };
    } catch (rejection) {
      if (request !== diffRequest) return;
      part.viewedDiff = {
        target,
        phase: "failed",
        failure: describe(rejection),
        diff: null,
        lines: [],
        revealed: reveal,
      };
    }
  }

  function openChange(file: ChangedFile): void {
    view.changes.selectedPath = file.path;
    void requestDiff(
      { path: file.path, oldPath: file.old_path, commit: null },
      false,
    );
  }

  async function openCommit(id: string): Promise<void> {
    const request = ++commitRequest;
    diffRequest += 1;
    view.history.viewedDiff = null;
    view.history.opened = {
      id,
      phase: "loading",
      failure: null,
      detail: null,
      selectedPath: null,
    };
    try {
      const detail = await service.commit(agentId, id);
      if (request !== commitRequest) return;
      view.history.opened = {
        id,
        phase: "ready",
        failure: null,
        detail,
        selectedPath: null,
      };
    } catch (rejection) {
      if (request !== commitRequest) return;
      view.history.opened = {
        id,
        phase: "failed",
        failure: describe(rejection),
        detail: null,
        selectedPath: null,
      };
    }
  }

  function openCommitFile(file: ChangedFile): void {
    const opened = view.history.opened;
    if (opened === null) return;
    opened.selectedPath = file.path;
    void requestDiff(
      { path: file.path, oldPath: file.old_path, commit: opened.id },
      false,
    );
  }

  /** Asks again for the one file or diff on show, with the user's reveal. */
  function revealFile(): void {
    const viewed = view.files.viewed;
    if (viewed !== null) void requestFile(viewed.path, true);
  }

  function revealDiff(): void {
    const part = view.tab === "history" ? view.history : view.changes;
    if (part.viewedDiff !== null)
      void requestDiff(part.viewedDiff.target, true);
  }

  return {
    view,
    showTab,
    refresh,
    toggleFolder,
    setExpanded,
    openEntry,
    openChange,
    openCommit,
    openCommitFile,
    showMoreCommits,
    revealFile,
    revealDiff,
  };
}

export type FileViewState = ReturnType<typeof createFileViewState>;
