// SPDX-License-Identifier: AGPL-3.0-or-later

import { defineConfig, devices } from "@playwright/test";

const previewUrl = "http://localhost:4173";

export default defineConfig({
  testDir: "e2e",
  webServer: {
    command: "bun run build && bun run preview",
    url: previewUrl,
    reuseExistingServer: !process.env.CI,
  },
  use: { baseURL: previewUrl },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
});
