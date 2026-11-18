import {useEffect} from 'react';

import {webAppAssetUrls} from './webAppAssets';

/**
 * How long the pointer must rest on a link before the download starts. A
 * passing sweep across a button should not cost the visitor a multi-megabyte
 * fetch, while a deliberate hover still warms the app before the click.
 */
const HOVER_INTENT_MS = 150;

/** A connection hint from the Network Information API, where it exists. */
type ConnectionHint = {
  saveData?: boolean;
  effectiveType?: string;
};

function connectionHint(): ConnectionHint {
  return (
    (navigator as Navigator & {connection?: ConnectionHint}).connection ?? {}
  );
}

/**
 * Whether this visit should spend bandwidth on a page the visitor has not
 * opened yet. Save-Data and second-generation mobile connections are treated as
 * a request not to prefetch; the app still loads normally when they click.
 */
function wantsSpeculativeDownload(): boolean {
  if (window.matchMedia?.('(prefers-reduced-data: reduce)').matches) {
    return false;
  }
  const connection = connectionHint();
  if (connection.saveData) return false;
  return (
    connection.effectiveType !== 'slow-2g' &&
    connection.effectiveType !== '2g'
  );
}

let warmStarted = false;

async function fetchIntoCache(url: string): Promise<void> {
  try {
    // Low priority keeps the 30+ MB WASM bundle behind anything the current
    // page still needs. Chromium understands `priority`; other engines ignore
    // the field.
    await fetch(url, {credentials: 'same-origin', priority: 'low'});
  } catch {
    // Best effort: a failed warm-up must never surface to the visitor.
  }
}

/**
 * Download the hosted browser app into the HTTP cache without starting it.
 *
 * The app is a single-page WASM bundle staged at `/app/`; its JavaScript and
 * WASM filenames are hashed, so the entry document is fetched first and read
 * for those assets. Nothing executes here, and the frame that `/play` mounts
 * later reuses the cached responses.
 */
export async function preloadWebApp(appUrl: string): Promise<void> {
  if (warmStarted || !wantsSpeculativeDownload()) return;
  warmStarted = true;
  try {
    const entry = new URL(appUrl, document.baseURI);
    const response = await fetch(entry, {credentials: 'same-origin'});
    if (!response.ok) return;
    const assets = webAppAssetUrls(
      await response.text(),
      entry.href,
      window.location.origin,
    );
    await Promise.all(assets.map(fetchIntoCache));
  } catch {
    // Leave the click path to report a real failure.
  }
}

function normalizedPath(url: URL): string {
  return url.pathname.replace(/\/+$/, '') || '/';
}

/**
 * Warm the hosted browser app when a visitor shows intent to open it, the way a
 * catalog site fetches the page behind a hovered button. The listener is
 * delegated from the document so the navbar Play link, the landing-page button,
 * the download button, and links inside the guide all behave the same without
 * each component opting in.
 *
 * `appUrl` and `playUrl` come from `useBaseUrl`, so a deployment under a
 * sub-path resolves correctly. The effect is skipped on `/play` itself: the
 * frame is already loading the app there.
 */
export function useWebAppPreload(appUrl: string, playUrl: string): void {
  useEffect(() => {
    const appPath = normalizedPath(new URL(appUrl, document.baseURI));
    const playPath = normalizedPath(new URL(playUrl, document.baseURI));
    if (normalizedPath(new URL(window.location.href)) === playPath) return;

    let armed = false;
    let timer: number | undefined;

    const disarm = () => {
      armed = false;
      if (timer !== undefined) {
        window.clearTimeout(timer);
        timer = undefined;
      }
    };

    const targetOf = (event: Event): URL | null => {
      const target = event.target;
      if (!(target instanceof Element)) return null;
      const anchor = target.closest('a[href]');
      if (!(anchor instanceof HTMLAnchorElement)) return null;
      try {
        return new URL(anchor.href, document.baseURI);
      } catch {
        return null;
      }
    };

    const armIfAppLink = (event: Event) => {
      const url = targetOf(event);
      if (!url) return;
      const path = normalizedPath(url);
      if (path !== appPath && path !== playPath) return;
      if (armed) return;
      armed = true;
      timer = window.setTimeout(() => {
        timer = undefined;
        void preloadWebApp(appUrl);
      }, HOVER_INTENT_MS);
    };

    const disarmWhenLeaving = (event: Event) => {
      if (!armed) return;
      const target = event.target;
      if (!(target instanceof Element)) return;
      const anchor = target.closest('a[href]');
      if (!(anchor instanceof Element)) return;
      const related = (event as PointerEvent).relatedTarget;
      // Moving between the link's own children is still hovering the link.
      if (related instanceof Node && anchor.contains(related)) return;
      disarm();
    };

    document.addEventListener('pointerover', armIfAppLink, {passive: true});
    document.addEventListener('pointerout', disarmWhenLeaving, {passive: true});
    document.addEventListener('focusin', armIfAppLink);
    document.addEventListener('focusout', disarmWhenLeaving);

    return () => {
      disarm();
      document.removeEventListener('pointerover', armIfAppLink);
      document.removeEventListener('pointerout', disarmWhenLeaving);
      document.removeEventListener('focusin', armIfAppLink);
      document.removeEventListener('focusout', disarmWhenLeaving);
    };
  }, [appUrl, playUrl]);
}
