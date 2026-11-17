---
title: Docs and screenshots
description: How this guide is written, how screenshots are produced, and what to check before changing a page.
sidebar_position: 1
---

This page is for people changing the documentation site. User-facing guides
live in `website/docs`; the internal plan is `docs/DOCS_WEBSITE_PLAN.md`.

## Documentation follows code

Update the docs in the same change as the code that alters user-visible
behavior. Do not leave a follow-up docs task, and do not merge a behavior change
while the guide still describes the old behavior.

Prose and screenshots have different cadences: prose changes with the code, and
the PNGs are refreshed in one pass when a release is prepared.

- Change the relevant page under `website/docs/`.
- Add or remove shots in `website/screenshots/manifest.json`, and fix alt text
  or captions that no longer match. Mark a new in-app shot `pending` until the
  release refresh.
- Leave existing PNGs, their dimensions, and `website/screenshots/emphasis.json`
  alone during ordinary work.
- Run `npm test` and `npm run check` in `website/`.
- Do not run `npm run screenshots` for an ordinary change.
- If no page covers the new behavior, add or extend a reference page.

A pure refactor does not need new prose, but stale text found along the way
should still be fixed. Between releases a committed screenshot may lag the UI;
the prose is the source of truth.

## Inspecting the app at both sizes

User-visible UI work is reviewed at both a desktop and a phone size. The rule
is recorded in `AGENTS.md` §9; it applies to every screen, picker, modal, empty
state, and error banner, not only to the two screens with phone screenshots.

- Render the phone pass and look at every image:

  ```sh
  cargo run -p caller-app --features headless --bin caller-headless -- target/mobile-shots mobile
  ```

  The images land in `target/mobile-shots/mobile/` and are not committed.
- Open the same states at 1280×800 (the desktop screenshot viewport) and at the
  360-point minimum window width.
- Check that no control is clipped, forms keep their labels, results and
  previews stack instead of squeezing into columns, and touch targets stay at
  least 44 points.
- If the live app is available, the MCP `resize` tool can switch an inspectable
  window between the two sizes without restarting it.

Fix what the phone pass reveals. Do not add a limitation that says a picker is
only usable in a wider window.

For the website's own mobile layout, build it and open the iOS preview:

```sh
cd website
npm run build
npm run preview:ios
```

That serves the production build in a 390×844 iPhone frame with an iOS user
agent. Use `--path /play/` to review the hosted browser app, or `--device ipad`
for tablet width. See `website/README.md` for the options. The preview does not
rebuild the site, and `/play` needs `npm run build:app` before the build.

## Writing style

- Second person, present tense.
- One action per step; start with a verb.
- Use the control names exactly as they appear in the app.
- Do not document a feature that is not in the current UI.
- A step's screenshot shows the result of that action, not a mockup.
- Do not include real tokens, usernames, device IDs, or third-party cards.

## Hosted browser app

The `/play` page embeds the WASM build that the docs workflow stages under
`website/static/app/`. `npm start` builds and stages it automatically; that
needs Rust, the `wasm32-unknown-unknown` target, and Trunk. If one is missing,
the build stops and prints the commands that install it.

For docs-only work, `npm run start:docs` skips the build. A tracked
`website/static/app/index.html` placeholder explains how to build the app, so
`/play` shows that explanation instead of a bare 404 when the build is absent.
`npm run build:app` replaces the placeholder; do not commit the staged build.

The app is built with the public URL `/app/`; keep that in step with
`baseUrl` in `docusaurus.config.ts` or set `CALLER_WEB_PUBLIC_URL`.

To review the hosted app at phone width, run
`npm run preview:ios -- --path /play/` after building the site.

## Screenshots

Screenshots are rendered by the real app through the headless entry point, then
annotated with PhotoCraft so the control a step talks about is visually marked.
The renderer uses `egui_kittest` with a silent transport, in-memory source cache,
and no preference file. It does not open a window, audio device, TTS voice, or
network connection.

Refresh them in one pass while preparing a release, not for every change:

```sh
cd website
npm run screenshots
```

That command:

1. renders the raw PNGs into `website/screenshots/raw/` (ignored by Git);
2. asks `photocraft-cli` to draw the focus ring from
   `website/screenshots/emphasis.json` and writes the final PNGs to
   `website/static/img/screens/`;
3. updates the manifest dimensions.

`npm run annotate` repeats only step 2 from the existing raw files. PhotoCraft is
found at the standard install location, in `PATH`, or through `PHOTOCRAFT_CLI`.

`website/screenshots/emphasis.json` stores one or more pixel rectangles per
ready shot, plus the expected image size. If the UI moves, update the rectangle
and the size; the annotation step fails rather than drawing the ring in the
wrong place. `npm test` checks that every ready shot has an entry and that every
rectangle is inside its image.

Until a shot is generated, `website/screenshots/manifest.json` marks it
`pending`. The `<Shot>` component renders a labelled placeholder rather than a
fake UI. Do not replace a placeholder with a screenshot of a mockup.

At release prep, when a shot is ready:

1. put the PNG at `website/static/img/screens/<id>.png`;
2. set the manifest entry's `status` to `ready` and add its dimensions;
3. run the screenshot checker and the Docusaurus build;
4. review the full-size image at desktop and phone widths.

`npm test` / `npm run check:shots` hashes every ready screenshot with SHA-256 and
fails if two files are byte-for-byte identical. Two views that look the same
must be merged, given a genuinely different state, or removed from the manifest
instead of being duplicated.

## MDX components

- `<Steps>` wraps a numbered list.
- `<Step shot="id">` renders an instruction and its screenshot.
- `<CommandStep command="…">` is for shell commands; it is not a UI step.
- `<Shot id="id" />` is for a standalone figure.
- `<DownloadButton platform="windows" />` is the release download button; add it
  wherever a page tells the reader to download the app. Pass `windows`, `linux`,
  `android`, `web`, `ios`, or `hosted`, or leave `platform` off to follow the
  visitor's operating system. On iOS and with `hosted`, the button opens the
  hosted browser app at `/play` instead, so it still resolves through the site's
  base URL. Every other button links straight at the release asset, so the page
  never sends the reader to GitHub. Add `variant="link"` when the build is an
  aside for technical readers: the page then renders one inline link with the
  version note instead of the filled button, and the reader's main action stays
  visually dominant.
- `<Figure>` is reserved for the few manual OS installer images.

A page should not use a raw `<img>` for an in-app screenshot.

## Before opening a pull request

Open pull requests against `dev`, not `main`. `dev` is the integration branch;
`main` advances only when a release is ready.

- [ ] The page renders at 390 px and 1280 px.
- [ ] Any app screen this change touches was inspected at 1280×800 and
      390×844, including its dialogs and error state.
- [ ] Every step has a shot or is an explicit command step.
- [ ] No broken links or anchors.
- [ ] No third-party content or real credentials.
- [ ] Any new or removed shot is recorded in the manifest; existing PNGs are not
      regenerated in this change.
- [ ] Dark and light themes both read correctly.
- [ ] `npm run build` passes in `website/`.

## Source material

The public docs are derived from the application code and the product
decisions in `docs/roadmap/locked-product-decisions.md`. When code and prose
disagree, the running app wins; update the prose, or fix the app if the behavior
is wrong.
