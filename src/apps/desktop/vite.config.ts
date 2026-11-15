// SPDX-License-Identifier: AGPL-3.0-or-later

import { fileURLToPath } from "node:url";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [tailwindcss(), svelte()],
  resolve: {
    alias: { $shared: fileURLToPath(new URL("./src/shared", import.meta.url)) },
  },
  clearScreen: false,
});
