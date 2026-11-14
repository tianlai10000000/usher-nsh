/** Test-only native catalog injection and independent OS registration oracle. */
import * as native from "usher/nsh";
export const fixture = (native as typeof native & { GlobalFixture: { attach(): unknown; reserve(): boolean; release(): void; available(): boolean; backgroundPress(key: string): string; active(): boolean } }).GlobalFixture;
