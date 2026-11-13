/** Runs button lifecycle journeys with the documented headless picker handler. */
import { registerButtonTests } from "./support/button-journeys";
registerButtonTests({
  buttons: "./fixtures/AsyncButtons.tsx",
  settings: "./fixtures/SettingsActionsPending.tsx",
  updates: "./fixtures/UpdateActionStates.tsx",
}, {
  prepare: app => { app.onPick(() => null); },
  cancelled: () => {},
});
