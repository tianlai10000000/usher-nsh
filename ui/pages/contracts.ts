/** Dependencies lent to page entries by the application composition root. */
import type { AppearanceStore } from "../state/appearance";
import type { WindowSession } from "../state/window";
import type { Navigation } from "../router";
export type PageProps = { updates?: import("../features/updates").Updates; appearance: AppearanceStore; session: WindowSession; navigation: Navigation };
