# website

User documentation for Contra Karaoke, built with Docusaurus 3.10.

## Status

The guide is written against the 0.1.0 development build (desktop + web MVP;
Android configured but device-testing pending). In-app screenshots are rendered
by the headless `contra-headless` binary and annotated with PhotoCraft. The
remaining placeholders are the manual Windows/Android installer dialogs.

The current manifest has 88 planned shots; 69 are ready. Ten pending in-app
shots are refreshed during release prep. The other nine are manual
Windows/Android/Linux installer and browser captures rather than app frames.

## Commands

```sh
npm install
npm start            # builds the app, then http://localhost:3000/Contra-karaoke/
npm run start:docs   # docs only: skips the browser build; /play shows a placeholder
npm run build:app    # requires Rust + Trunk; stages the WASM app under static/app
npm test             # screenshot manifest, uniqueness, and emphasis regions
npm run check        # npm test + production build
npm run screenshots  # release prep only: headless renderer + PhotoCraft annotation
npm run annotate     # release prep only: re-apply PhotoCraft to existing raw PNGs
npm run build
npm run serve
npm run clear
```

`npm start` runs `npm run build:app` through its `prestart` hook. If Rust, the
`wasm32-unknown-unknown` target, or Trunk is missing, the build stops with the
commands to install what is missing; use `npm run start:docs` when you are only
changing prose or screenshots.

The project base URL is `/Contra-karaoke/` for GitHub Pages. Test the built site
under that path, not only at the server root.

## Hosted browser app

The site serves a copy of the WASM app at `/Contra-karaoke/app/`, embedded by
the `/play` page. `npm run build:app` runs Trunk against `crates/web`, builds
with the public URL `/Contra-karaoke/app/`, and copies `dist/web` to
`static/app/`. `npm start` runs it automatically; the docs workflow runs it
before `npm run check`.

The tracked `static/app/index.html` is a hand-written placeholder that explains
how to build the app, so `/play` degrades to that explanation instead of a bare
404 when the build is absent. `build:app` replaces it, and the other files in
`static/app/` are generated and ignored by Git; do not commit the staged build,
even though the placeholder shares its path. A plain `npm run build` without
Rust therefore produces a site whose `/play` frame shows the placeholder.
Override the public URL with `CONTRA_WEB_PUBLIC_URL` if you deploy the site at a
different base path.

## Screenshot manifest

`website/screenshots/manifest.json` lists every planned shot ID with its alt
text and caption. A shot renders as an image when its `status` is `ready` and
`static/img/screens/<id>.png` exists. Until then it renders as a pending
placeholder so the prose can be reviewed without fake UI.
