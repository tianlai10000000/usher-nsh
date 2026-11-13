/** Adapts untrusted host links and synchronous back requests to guarded navigation. */
import type { Navigation, Target } from "./index";
export interface LinkPolicy { schemes: readonly string[]; hosts: readonly string[] }
export interface RoutingHost {
  onOpen(listener: (url: string) => void): () => void;
  onBack(listener: () => boolean): () => void;
  observeReady(listener: () => void): () => void;
  ownsBack(): boolean;
}
/** Preserve the raw path for the Rust parser: URL normalization must not hide traversal. */
export function linkTarget(url: string, policy: LinkPolicy): Target | null {
  if (/[\s\\#]/.test(url)) return null;
  const match = /^([a-z][a-z0-9+.-]*):\/\/([^/?#]+)(.*)$/i.exec(url);
  if (!match || !policy.schemes.includes(match[1].toLowerCase()) || !policy.hosts.includes(match[2].toLowerCase())) return null;
  const location = match[3].startsWith("/") ? match[3] : `/${match[3]}`;
  return { kind: "path", location };
}
/** Pending launch events coalesce until the projection is live. Disposal fences queued
 * events, including callbacks already handed to the host before listener removal. */
export function bindHostRouting(router: Pick<Navigation, "status" | "resolve" | "push" | "canGoBack" | "goBack">, host: RoutingHost, policy: LinkPolicy) {
  let disposed = false;
  let queued: Target | null = null;
  const flush = () => {
    if (disposed || router.status() !== "live" || !queued) return;
    const target = queued; queued = null;
    try { router.resolve(target); void router.push(target).catch(() => {}); }
    catch { /* Invalid external input cannot mutate authoritative history. */ }
  };
  const stopOpen = host.onOpen(url => {
    if (disposed) return;
    const target = linkTarget(url, policy);
    if (target) { queued = target; flush(); }
  });
  const stopBack = host.onBack(() => {
    if (disposed || !host.ownsBack() || router.status() !== "live" || !router.canGoBack()) return false;
    // Consume synchronously; asynchronous guards decide whether Rust history moves.
    void router.goBack().catch(() => {});
    return true;
  });
  const stopReady = host.observeReady(flush);
  flush();
  return () => {
    if (disposed) return;
    disposed = true; queued = null; stopOpen(); stopBack(); stopReady();
  };
}
