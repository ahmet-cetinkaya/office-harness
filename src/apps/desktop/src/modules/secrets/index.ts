// SPDX-License-Identifier: AGPL-3.0-or-later

export {
  createSecretsController,
  INITIAL_SECRETS,
  type FailureText,
  type SecretsController,
  type SecretsSnapshot,
} from "./state/secrets-controller";
export { default as SecretsDialog } from "./components/secrets-dialog.svelte";
