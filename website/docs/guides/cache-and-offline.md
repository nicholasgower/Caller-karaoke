---
title: Cache and offline
description: How Contra Karaoke caches dances, tunes, and searches, how stale content is used, and how to clear the cache or configure the web proxy.
sidebar_position: 9
---

Contra Karaoke fetches dances from ContraDB and tunes from ABC Session Tunes,
then caches the raw responses locally: files on the desktop and Android, and
`localStorage` in the browser. The two bundled defaults work without a network.

## Cache lifetimes

| Content | Lifetime |
| --- | --- |
| Search results | 7 days |
| Dance details | 30 days |
| Tune details | 30 days |

A search or detail request reads the cache first. If the cache entry is fresh,
the app uses it without a request. If it is stale, the app requests the source
and may replace the entry.

## What happens offline

If a request fails, the app looks for a stale cache entry. If one exists, it
parses and uses that content. **There is no offline banner in 0.1.0**; the
fallback is silent.

If no cached content exists, the error appears in a red panel at the bottom of
the window. Select **Dismiss** to close it. The currently loaded dance and tune
stay in place.

<Shot id="error-network" />

The bundled **Another Last Dance** and **Sandy Boys** are always available. Other
dances and tunes are only available offline if they were fetched and cached
earlier.

## Clear the source cache

<Steps>
  <Step shot="prefs-cache-confirm">
    Open **Preferences → Maintenance** and select **Clear source cache**. The
    button changes to **Confirm clear cache** and **Cancel**.
  </Step>
  <Step shot="prefs-cache-cleared">
    Select **Confirm clear cache**. The status line reports `Cache cleared`.
  </Step>
</Steps>

Clearing the cache does not remove preferences, the bundled defaults, or a
loaded SoundFont.

There is no cache-size display or manual refresh button in the current UI. Use
the normal search action to fetch fresh content after clearing.

## Web data proxy

Desktop and Android builds fetch the sources directly. A browser build cannot
do that because the upstream sites do not allow the app origin, so the
repository includes a Cloudflare Worker proxy in `proxy/`.

<Steps>
  <Step shot="prefs-proxy">
    In **Preferences → Web data proxy**, enter the deployed proxy **Proxy URL**
    and, if the deployment requires one, a **Token**.
  </Step>
</Steps>

The worker forwards only the two allowlisted upstream hosts and the API paths
the app uses. Do not point the app at an open proxy. The token is stored locally
with your preferences; do not share a screenshot of a real token.

## What is not cached

- Audio renders are not stored; the app re-renders on Play after a change.
- Your current transport position is not persisted.
- A failed response is not cached as a valid result.

Next: [Themes and accessibility](/docs/guides/themes-and-accessibility).
