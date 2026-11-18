/**
 * Find the JavaScript and WebAssembly files that the hosted browser build loads.
 *
 * Trunk writes hashed asset names into `static/app/index.html` at build time, so
 * the site cannot name them in advance. Reading them back from the entry
 * document keeps the hover preload working across builds and in the placeholder
 * case, where there is nothing to fetch.
 *
 * This is deliberately a regex over `src` and `href` attributes rather than a
 * DOM parser: it runs during a hover, it only ever sees Trunk's own generated
 * document, and it stays importable from the Node check without a browser.
 * Cross-origin URLs and every other file type are ignored.
 */
export function webAppAssetUrls(
  html: string,
  appUrl: string,
  origin: string,
): string[] {
  const base = new URL(appUrl, origin);
  const found = new Set<string>();
  for (const match of html.matchAll(/\b(?:src|href)="([^"]+)"/gi)) {
    let url: URL;
    try {
      url = new URL(match[1], base);
    } catch {
      continue;
    }
    if (url.origin !== origin) continue;
    if (!/\.(?:m?js|wasm)$/i.test(url.pathname)) continue;
    found.add(url.href);
  }
  return Array.from(found);
}
