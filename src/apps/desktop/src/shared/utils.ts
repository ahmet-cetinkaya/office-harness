// SPDX-License-Identifier: AGPL-3.0-or-later

import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...classValues: ClassValue[]): string {
  return twMerge(clsx(classValues));
}

const TIMESTAMP_FORMAT = new Intl.DateTimeFormat("en-GB", {
  dateStyle: "medium",
  timeStyle: "medium",
});

/** A Unix time in milliseconds as local date and time text. */
export function formatTimestamp(unixMillis: number): string {
  return TIMESTAMP_FORMAT.format(new Date(unixMillis));
}
