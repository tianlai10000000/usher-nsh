/** Exhaustive page-key registry; business routes are declared only in routes.json. */
import type { JSX } from "nsh/jsx-runtime";
import type { PageKey } from "./routes.generated";
import type { PageProps } from "./contracts";
import { HomePage } from "./home/Page";
import { SettingsEntry } from "./settings/Page";
import { NotFoundPage } from "./not-found/Page";
export const pages = { home: HomePage, settings: SettingsEntry, "not-found": NotFoundPage } satisfies Record<PageKey, (props: PageProps) => JSX.Element>;
