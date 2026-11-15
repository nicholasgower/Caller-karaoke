# website

User documentation for Contra Karaoke, built with Docusaurus 3.10.

## Status

The guide is written against the 0.1.0 development build (desktop + web MVP;
Android configured but device-testing pending). In-app screenshots are rendered
by the headless `contra-headless` binary and annotated with PhotoCraft. The
remaining placeholders are the manual Windows/Android installer dialogs.

The current manifest has 83 planned shots; 69 are ready. Five pending in-app
shots are refreshed during release prep. The other nine are manual
Windows/Android/Linux installer captures rather than app frames.

## Commands

```sh
npm install
npm run build:app  # requires Rust + Trunk; stages the WASM app under static/app
npm start      # http://localhost:3000/Contra-karaoke/
npm test       # screenshot manifest, uniqueness, and emphasis regions
npm run check  # npm test + production build
npm run screenshots  # release prep only: headless renderer + PhotoCraft annotation
npm run annotate     # release prep only: re-apply PhotoCraft to existing raw PNGs
npm run build
npm run serve
npm run clear
```

The project base URL is `/Contra-karaoke/` for GitHub Pages. Test the built site
under that path, not only at the server root.

## Hosted browser app

The site serves a copy of the WASM app at `/Contra-karaoke/app/`, embedded by
the `/play` page. `npm run build:app` runs Trunk against `crates/web`, builds
with the public URL `/Contra-karaoke/app/`, and copies `dist/web` to
`static/app/`. That folder is generated and ignored by Git; the docs workflow
builds it before `npm run check`, so a plain `npm run build` without Rust
produces a site with a broken `/play` frame. Override the public URL with
`CONTRA_WEB_PUBLIC_URL` if you deploy the site at a different base path.

## Screenshot manifest

`website/screenshots/manifest.json` lists every planned shot ID with its alt
text and caption. A shot renders as an image when its `status` is `ready` and
`static/img/screens/<id>.png` exists. Until then it renders as a pending
placeholder so the prose can be reviewed without fake UI.
