/** Owns background image actions and preview projections over borrowed settings and window state. */
import { Picker } from "nsh";
import backgroundConfig from "../../config/background.json";
import type { MessageKey } from "../../i18n/index";
import type { WindowSession } from "../window/index";
import type { SettingsView, SettingsOperations } from "../settings/operations";

/** Only the picker capability is injected; file bytes are read after metadata validation. */
export interface BackgroundPicker {
  pickImages(options: { maxDimension: number }): Promise<readonly {
    file: { size: number; name: string; bytes(): Promise<Uint8Array> };
  }[]>;
}
export function createBackgroundModule(
  settings: SettingsView,
  session: Pick<WindowSession, "value" | "begin" | "finish" | "change" | "preview">,
  operations: Pick<SettingsOperations, "sync" | "busy" | "disposed">,
  picker: BackgroundPicker = Picker,
) {
  const snapshot = settings.value;
  const { sync, busy } = operations;
  const pickingBackground = () => session.value().operations.picker?.active === true;
  const backgroundProblem = () => session.value().backgroundProblem as MessageKey | undefined;
  const backgroundImageId = settings.select(value => value.backgroundImage?.id);
  let picking = false;
  async function chooseBackgroundImage() {
    if (picking || pickingBackground() || busy() || operations.disposed()) return;
    picking = true;
    let ticket: string | undefined;
    try {
      ticket = await session.begin("picker", "running");
      if (operations.disposed()) return;
      await session.change({ kind: "background_problem", value: null });
      const [picked] = await picker.pickImages({ maxDimension: backgroundConfig.pickerMaxDimension });
      if (!picked || operations.disposed()) return;
      if (picked.file.size > backgroundConfig.maxImageBytes) {
        await session.change({ kind: "background_problem", value: "background.tooLarge" });
        return;
      }
      const bytes = await picked.file.bytes();
      if (!operations.disposed()) await sync(() => settings.dispatch({ type: "importBackground", name: picked.file.name, bytes }));
    } catch (cause) {
      if (!ticket) throw cause;
      if (!operations.disposed()) await session.change({ kind: "background_problem", value: "background.pickFailed" });
    } finally {
      picking = false;
      if (ticket && !operations.disposed()) await session.finish("picker", ticket, "idle");
    }
  }

  async function setBackgroundImageOpacity(value: number) {
    const receipt = await session.preview(true, value);
    if (receipt.kind !== "ticket") throw new Error("window_invalid_receipt");
    const ticket = receipt.ticket;
    try { return await sync(() => settings.dispatch({ type: "opacity", value })); }
    finally { if (!operations.disposed()) await session.preview(true, null, ticket); }
  }

  async function setContentBackgroundTransparency(value: number) {
    const receipt = await session.preview(false, value);
    if (receipt.kind !== "ticket") throw new Error("window_invalid_receipt");
    const ticket = receipt.ticket;
    try { return await sync(() => settings.dispatch({ type: "transparency", value })); }
    finally { if (!operations.disposed()) await session.preview(false, null, ticket); }
  }

  return {
    backgroundImage: () => snapshot().backgroundImage,
    backgroundImageId,
    backgroundImageOpacity: () => session.value().opacityPreview ?? snapshot().backgroundImageOpacity,
    contentBackgroundTransparency: () => session.value().transparencyPreview ?? snapshot().contentBackgroundTransparency,
    previewBackgroundImageOpacity: (value: number) => void session.preview(true, value),
    previewContentBackgroundTransparency: (value: number) => void session.preview(false, value),
    setBackgroundImageOpacity, setContentBackgroundTransparency, chooseBackgroundImage,
    clearBackgroundImage: () => sync(() => settings.dispatch({ type: "clearBackground" })),
    pickingBackground, backgroundProblem,
    reportBackgroundLoadFailure: (failed: boolean) => void session.change({ kind: "background_problem", value: failed ? "background.loadFailed" : null }),
  };
}
