/** Dependencies lent to page entries by the application composition root. */
import type { AppState } from "../state/app";
import type { WindowSession } from "../state/window/index";
import type { Navigation } from "../router/index";
export type PageProps = { shortcuts?: import("../shortcuts").Shortcuts; updates?: import("../features/updates").Updates; appearance: AppState; session: WindowSession; navigation: Navigation };
