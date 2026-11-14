/** Validates persisted language preferences at the transport boundary. */
import { requireValue, string } from "../../state-runtime/decode";
import { isLocalePreference } from "../../i18n/index";
export function decodeLanguage(value: unknown) {
  const language = string(value); requireValue(isLocalePreference(language)); return language;
}
