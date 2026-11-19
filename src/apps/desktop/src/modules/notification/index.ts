// SPDX-License-Identifier: AGPL-3.0-or-later

export {
  createNotificationController,
  INITIAL_NOTIFICATIONS,
  type NotificationSnapshot,
} from "./state/notification-store";
export { default as NotificationDialog } from "./components/notification-dialog.svelte";
export { default as NotificationLiveRegion } from "./components/notification-live-region.svelte";
export { default as UnreadButton } from "./components/unread-button.svelte";
export {
  rowText,
  unreadCount,
  type NotificationRow,
} from "./models/notification";
export { readLastView } from "./services/last-view-service";
export { resolveLastView, type LastView } from "./models/last-view";
export {
  createOsNotificationService,
  type OsNotificationService,
} from "./services/os-notification";
