---
title: Docs and screenshots
description: How this guide is written, how screenshots are produced, and what to check before changing a page.
sidebar_position: 1
---

This page is for people changing the documentation site. User-facing guides
live in `website/docs`; the internal plan is `docs/DOCS_WEBSITE_PLAN.md`.

## Writing style

- Second person, present tense.
- One action per step; start with a verb.
- Use the control names exactly as they appear in the app.
- Do not document a feature that is not in the current UI.
- A step's screenshot shows the result of that action, not a mockup.
- Do not include real tokens, usernames, device IDs, or third-party cards.

## Screenshots

Screenshots are rendered by the real app through the headless entry point, then
annotated with PhotoCraft so the control a step talks about is visually marked.
The renderer uses `egui_kittest` with a silent transport, in-memory source cache,
and no preference file. It does not open a window, audio device, TTS voice, or
network connection.

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

When a shot is ready:

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
- `<Figure>` is reserved for the few manual OS installer images.

A page should not use a raw `<img>` for an in-app screenshot.

## Before opening a pull request

- [ ] The page renders at 390 px and 1280 px.
- [ ] Every step has a shot or is an explicit command step.
- [ ] No broken links or anchors.
- [ ] No third-party content or real credentials.
- [ ] Screenshots match the current UI.
- [ ] Dark and light themes both read correctly.
- [ ] `npm run build` passes in `website/`.

## Source material

The public docs are derived from the application code and the product
decisions in `ROADMAP.md`. When code and prose disagree, the running app wins;
update the prose, or fix the app if the behavior is wrong.
