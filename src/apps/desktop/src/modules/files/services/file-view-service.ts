// SPDX-License-Identifier: AGPL-3.0-or-later

import type { CoreClient } from "../../../core/api";
import type {
  ChangeSet,
  CommitDetail,
  DiffTarget,
  FileDiff,
  FileList,
  FileRead,
  HistoryPage,
} from "../models/file-view";

/** The typed face of the six `agent.files.*` requests (contract 0016). */
export function createFileViewService(client: CoreClient) {
  const send = (kind: string, fields: Record<string, unknown>) =>
    client.sendRequest(kind, fields);

  return {
    list: async (agentId: string, path: string) =>
      (await send("agent.files.list", {
        agent_id: agentId,
        path,
      })) as FileList,
    read: async (agentId: string, path: string, reveal: boolean) =>
      (await send("agent.files.read", {
        agent_id: agentId,
        path,
        reveal,
      })) as FileRead,
    changes: async (agentId: string) =>
      (await send("agent.files.changes", { agent_id: agentId })) as ChangeSet,
    diff: async (agentId: string, target: DiffTarget, reveal: boolean) =>
      (await send("agent.files.diff", {
        agent_id: agentId,
        path: target.path,
        ...(target.oldPath === null ? {} : { old_path: target.oldPath }),
        ...(target.commit === null ? {} : { commit: target.commit }),
        reveal,
      })) as FileDiff,
    history: async (agentId: string, skip: number) =>
      (await send("agent.files.history", {
        agent_id: agentId,
        skip,
      })) as HistoryPage,
    commit: async (agentId: string, commit: string) =>
      (await send("agent.files.commit", {
        agent_id: agentId,
        commit,
      })) as CommitDetail,
  };
}

export type FileViewService = ReturnType<typeof createFileViewService>;
