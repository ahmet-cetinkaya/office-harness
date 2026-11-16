// SPDX-License-Identifier: AGPL-3.0-or-later

import { mount } from "svelte";
import App from "./app.svelte";
import "./styles/app.css";

const root = document.getElementById("app");
if (!root) {
  throw new Error("The page has no #app element to mount into.");
}

mount(App, { target: root });
