import en from "../../i18n/messages/en.json";
import zhHans from "../../i18n/messages/zh-Hans.json";

const catalogs = {
  en,
  "zh-Hans": zhHans,
} satisfies Record<string, typeof en>;

type Locale = keyof typeof catalogs;
export type LocalePreference = "system" | Locale;
export type MessageKey = keyof typeof en;
type MessageParams = Readonly<Record<string, string | number>>;
export type Translate = (key: MessageKey, params?: MessageParams) => string;

export function isLocalePreference(value: string): value is LocalePreference {
  return value === "system" || Object.hasOwn(catalogs, value);
}

export function resolveLocale(preference: LocalePreference, systemLocale: string): Locale {
  if (preference !== "system") return preference;

  const [language, subtag] = systemLocale.trim().toLowerCase().replaceAll("_", "-").split("-");
  if (language !== "zh") return "en";
  if (subtag?.length === 4) return subtag === "hans" ? "zh-Hans" : "en";
  if (subtag === "tw" || subtag === "hk" || subtag === "mo") return "en";
  return "zh-Hans";
}

export function translate(
  preference: LocalePreference,
  systemLocale: string,
  key: MessageKey,
  params?: MessageParams,
): string {
  const locale = resolveLocale(preference, systemLocale);
  const template = catalogs[locale][key];
  if (params === undefined) return template;

  return template.replace(/\{(\w+)\}/g, (placeholder, name: string) => {
    if (!Object.hasOwn(params, name)) return placeholder;
    const value = params[name];
    return typeof value === "number" ? new Intl.NumberFormat(locale).format(value) : value;
  });
}
