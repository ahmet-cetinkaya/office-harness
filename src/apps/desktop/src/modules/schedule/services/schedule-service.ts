// SPDX-License-Identifier: AGPL-3.0-or-later

import type { CoreClient } from "../../../core/api";
import {
  readHistory,
  readSchedule,
  readScheduleList,
  type RunRecord,
  type Schedule,
  type ScheduleList,
} from "../models/schedule";

const MALFORMED_ANSWER =
  "The server answered with a schedule that is unreadable.";

function scheduleOf(answer: unknown): Schedule {
  const schedule = readSchedule(
    typeof answer === "object" && answer !== null
      ? (answer as Record<string, unknown>).schedule
      : null,
  );
  if (schedule === null) throw new Error(MALFORMED_ANSWER);
  return schedule;
}

/** The typed face of the `schedule.*` requests (0025). */
export function createScheduleService(client: CoreClient) {
  return {
    list: async (): Promise<ScheduleList> =>
      readScheduleList(await client.sendRequest("schedule.list")),
    save: async (fields: Record<string, unknown>): Promise<Schedule> =>
      scheduleOf(await client.sendRequest("schedule.save", fields)),
    switchTo: async (scheduleId: string, on: boolean): Promise<Schedule> =>
      scheduleOf(
        await client.sendRequest("schedule.switch", {
          schedule_id: scheduleId,
          on,
        }),
      ),
    runNow: async (scheduleId: string): Promise<void> => {
      await client.sendRequest("schedule.run_now", {
        schedule_id: scheduleId,
      });
    },
    remove: async (scheduleId: string): Promise<void> => {
      await client.sendRequest("schedule.delete", { schedule_id: scheduleId });
    },
    history: async (scheduleId: string): Promise<RunRecord[]> =>
      readHistory(
        await client.sendRequest("schedule.history", {
          schedule_id: scheduleId,
        }),
      ),
  };
}
