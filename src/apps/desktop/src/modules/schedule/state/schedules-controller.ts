// SPDX-License-Identifier: AGPL-3.0-or-later

import {
  toCoreRequestError,
  type CoreClient,
  type ServerEvent,
} from "../../../core/api";
import {
  EMPTY_SCHEDULE_LIST,
  readSchedule,
  saveFields,
  type FieldProblem,
  type RunRecord,
  type Schedule,
  type ScheduleDraft,
  type ScheduleList,
} from "../models/schedule";
import { createScheduleService } from "../services/schedule-service";

export interface SchedulesSnapshot extends ScheduleList {
  phase: "closed" | "loading" | "ready" | "failed";
  /** Why the list did not load, or null. */
  failure: string | null;
  /** Why the last switch, Run now, or delete was refused, or null. */
  actionFailure: string | null;
  /** The newest run of each schedule that has run, by schedule id. */
  lastRuns: Record<string, RunRecord>;
}

/** What a save answered: it worked, or why not, field by field. */
export type SaveOutcome =
  { saved: true } | { saved: false; problems: FieldProblem[]; message: string };

/** The history of one schedule, while its view is open. */
export interface HistorySnapshot {
  scheduleId: string;
  runs: RunRecord[];
  failure: string | null;
  isLoading: boolean;
}

export const INITIAL_SCHEDULES: SchedulesSnapshot = {
  ...EMPTY_SCHEDULE_LIST,
  phase: "closed",
  failure: null,
  actionFailure: null,
  lastRuns: {},
};

const EVENT_SCHEDULE_CHANGED = "schedule.changed";
const EVENT_SCHEDULE_REMOVED = "schedule.removed";
const REQUEST_FAILED = "The request failed.";

/** Puts the schedule in the list by its id, in the list's name order. */
export function upsertSchedule(
  schedules: Schedule[],
  schedule: Schedule,
): Schedule[] {
  const rest = schedules.filter((known) => known.id !== schedule.id);
  return [...rest, schedule].sort((first, second) =>
    first.name.localeCompare(second.name),
  );
}

/**
 * Owns the schedules while the Schedules dialog is open: the list, the last
 * run of each, the history of one, and the changes that the person makes. The
 * server's events keep it current; a row is replaced by its id, never added
 * twice.
 */
export function createSchedulesController(client: CoreClient) {
  const service = createScheduleService(client);
  let snapshot: SchedulesSnapshot = INITIAL_SCHEDULES;
  let history: HistorySnapshot | null = null;
  let latestRead = 0;
  const listeners = new Set<(snapshot: SchedulesSnapshot) => void>();
  const historyListeners = new Set<(history: HistorySnapshot | null) => void>();

  function update(changes: Partial<SchedulesSnapshot>): void {
    snapshot = { ...snapshot, ...changes };
    for (const listener of listeners) listener(snapshot);
  }

  function setHistory(next: HistorySnapshot | null): void {
    history = next;
    for (const listener of historyListeners) listener(history);
  }

  function describe(rejection: unknown): string {
    return toCoreRequestError(rejection).message || REQUEST_FAILED;
  }

  const isClosed = (): boolean => snapshot.phase === "closed";

  async function readLastRun(scheduleId: string): Promise<void> {
    try {
      const [newest] = await service.history(scheduleId);
      if (isClosed() || newest === undefined) return;
      update({ lastRuns: { ...snapshot.lastRuns, [scheduleId]: newest } });
    } catch {
      // The row then shows no last run; the history view tells the failure.
    }
  }

  async function refresh(): Promise<void> {
    if (isClosed()) return;
    const read = ++latestRead;
    try {
      const list = await service.list();
      if (read !== latestRead || isClosed()) return;
      update({ ...list, phase: "ready", failure: null });
      await Promise.all(list.schedules.map((item) => readLastRun(item.id)));
    } catch (rejection) {
      if (read !== latestRead || isClosed()) return;
      update({ phase: "failed", failure: describe(rejection) });
    }
  }

  function applyChanged(schedule: Schedule): void {
    update({ schedules: upsertSchedule(snapshot.schedules, schedule) });
    void readLastRun(schedule.id);
    if (history?.scheduleId === schedule.id) void loadHistory(schedule.id);
  }

  function applyRemoved(scheduleId: string): void {
    update({
      schedules: snapshot.schedules.filter((item) => item.id !== scheduleId),
      lastRuns: Object.fromEntries(
        Object.entries(snapshot.lastRuns).filter(([id]) => id !== scheduleId),
      ),
    });
    if (history?.scheduleId === scheduleId) setHistory(null);
  }

  function handleEvent(event: ServerEvent): void {
    if (isClosed()) return;
    if (event.type === EVENT_SCHEDULE_CHANGED) {
      const schedule = readSchedule(event.schedule);
      if (schedule !== null) applyChanged(schedule);
    }
    if (event.type === EVENT_SCHEDULE_REMOVED) {
      if (typeof event.id === "string") applyRemoved(event.id);
    }
  }

  /** Runs one request on a schedule; resolves to the refusal, or null. */
  async function act(request: () => Promise<void>): Promise<string | null> {
    update({ actionFailure: null });
    try {
      await request();
      return null;
    } catch (rejection) {
      const failure = describe(rejection);
      update({ actionFailure: failure });
      return failure;
    }
  }

  async function loadHistory(scheduleId: string): Promise<void> {
    setHistory({ scheduleId, runs: [], failure: null, isLoading: true });
    try {
      const runs = await service.history(scheduleId);
      if (history?.scheduleId !== scheduleId) return;
      setHistory({ scheduleId, runs, failure: null, isLoading: false });
    } catch (rejection) {
      if (history?.scheduleId !== scheduleId) return;
      setHistory({
        scheduleId,
        runs: [],
        failure: describe(rejection),
        isLoading: false,
      });
    }
  }

  return {
    get snapshot(): SchedulesSnapshot {
      return snapshot;
    },

    /** Calls back on each change, starting with the current snapshot. */
    subscribe(listener: (snapshot: SchedulesSnapshot) => void): () => void {
      listeners.add(listener);
      listener(snapshot);
      return () => listeners.delete(listener);
    },

    subscribeHistory(
      listener: (history: HistorySnapshot | null) => void,
    ): () => void {
      historyListeners.add(listener);
      listener(history);
      return () => historyListeners.delete(listener);
    },

    /** Listens for the server's events. Call it once. */
    watch: async (): Promise<void> => {
      try {
        await client.watchEvents(handleEvent);
      } catch {
        // Without events the dialog still reads the list when it opens.
      }
    },

    /** The dialog opened: events and reads count from now on. */
    open: async (): Promise<void> => {
      update({ ...INITIAL_SCHEDULES, phase: "loading" });
      await refresh();
    },

    /** The dialog closed: events no longer change the list. */
    close(): void {
      setHistory(null);
      update(INITIAL_SCHEDULES);
    },

    refresh,

    /** Saves a new schedule, or an edit when the id is given. */
    async save(
      draft: ScheduleDraft,
      scheduleId: string | null,
    ): Promise<SaveOutcome> {
      try {
        applyChanged(await service.save(saveFields(draft, scheduleId)));
        return { saved: true };
      } catch (rejection) {
        const error = toCoreRequestError(rejection);
        return {
          saved: false,
          problems: error.fields ?? [],
          message: error.message || REQUEST_FAILED,
        };
      }
    },

    switchTo: (scheduleId: string, on: boolean): Promise<string | null> =>
      act(async () => applyChanged(await service.switchTo(scheduleId, on))),

    runNow: (scheduleId: string): Promise<string | null> =>
      act(() => service.runNow(scheduleId)),

    remove: (scheduleId: string): Promise<string | null> =>
      act(async () => {
        await service.remove(scheduleId);
        applyRemoved(scheduleId);
      }),

    openHistory: loadHistory,

    closeHistory(): void {
      setHistory(null);
    },
  };
}

export type SchedulesController = ReturnType<typeof createSchedulesController>;
