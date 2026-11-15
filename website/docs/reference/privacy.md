---
title: Privacy
description: What Contra Karaoke stores, what it sends, and what it does not collect.
sidebar_position: 4
---

Contra Karaoke has no accounts, no telemetry, and no analytics. The application
does not send usage data to the project.

## What stays on your machine

- **Preferences** — role terms, aliases, instruments, theme, tempo, lead
  settings, TTS settings, proxy settings, and the last selected source
  references. They are stored with a schema version in the standard per-user
  configuration directory, or in `localStorage` in the browser build.
- **Source cache** — search responses, dance details, and tune details, stored
  locally with timestamps. Native and Android builds use files; the browser uses
  `localStorage`. The cache is cleared manually from Preferences.
- **SoundFont** — if you load one, the app stores its display name and path in
  preferences. The file itself remains where you put it. The bundled subset
  ships inside the application and stores nothing.
- **Neural voice cache** — on desktop, the Kokoro model and the selected voice
  style file are stored in the app cache after the first download.

## What goes over the network

The app contacts only the sources you use:

- `contradb.com` for dance search and details;
- `backend.abcsessiontunes.com` for tune search and details;
- a configured web proxy, if you set one;
- Hugging Face, once, to download the Kokoro model and voice files for the
  desktop neural voice.

There are no analytics endpoints, crash reports, or update checks. The app does
not fetch a page unless a search or selection asks for it; it does not crawl a
site. After the neural voice is cached, speech synthesis is local.

## Proxy tokens

A web proxy token is stored locally with your preferences and sent as an
`X-Proxy-Token` header to the proxy. It is not logged by the application.
Documentation screenshots use a fake token or leave the field blank.

## The documentation site

This site is static and hosted on GitHub Pages. It has no cookies, analytics,
advertising, or third-party fonts. Search is compiled into the site, so queries
do not leave the browser. GitHub, as the host, receives normal request logs;
see GitHub's privacy statement for details.

The [hosted browser copy](/play) is the same WASM application and follows the
same privacy rules: preferences and cached source responses stay in this site's
`localStorage`, and the app sends no telemetry. GitHub sees requests for the
application files just as it sees requests for the rest of the site.

## Third-party content

Dances and tunes remain the work of their contributors. The app caches them for
your use but does not redistribute a database. The bundled defaults and the
Fluid R3 GM soundfont subset are attributed on this site and in `CREDITS.md`. A
SoundFont you load is your responsibility, including its license.

## Reporting a privacy issue

Open an issue in the project repository with the screen and action involved. Do
not include a real proxy token, a private dance URL, or a file path in a public
report.
