# website

User documentation for Caller Karaoke, built with Docusaurus 3.10.

## Status

The guide is written against the 0.1.0 development build (desktop + web MVP;
Android configured but device-testing pending). In-app screenshots are rendered
by the headless `caller-headless` binary, which also draws the focus rings from
the widgets each shot names. The remaining placeholders are the manual Windows,
Linux, web, and Android installation captures.

The current manifest has 90 planned shots; 80 are ready. Three pending in-app
shots cover the slow ending and the tune set; the headless renderer does not
capture those states yet. The other seven are manual
Windows/Android/Linux/web installer captures rather than app frames.

## Commands

```sh
npm install
npm start            # builds the app, then http://localhost:3000/
npm run start:docs   # docs only: skips the browser build; /play shows a placeholder
npm run build:app    # requires Rust + Trunk; stages the WASM app under static/app
npm test             # screenshot manifest, uniqueness, and preload
npm run check        # npm test + production build
npm run preview:ios  # serve website/build in an iPhone frame (build first)
npm run screenshots  # release prep only: headless renderer with in-app callouts
npm run build
npm run serve
npm run clear
```

`npm start` runs `npm run build:app` through its `prestart` hook. If Rust, the
`wasm32-unknown-unknown` target, or Trunk is missing, the build stops with the
commands to install what is missing; use `npm run start:docs` when you are only
changing prose or screenshots.

The site is served from the root of <https://callerkaraoke.nicholasgower.com/>
(`baseUrl: '/'`). Test the built site at the server root. `static/CNAME` pins
the GitHub Pages custom domain; keep it in step with `url` in
`docusaurus.config.ts`.

## Hosted browser app

The site serves a copy of the WASM app at `/app/`, embedded by
the `/play` page. `npm run build:app` runs Trunk against `crates/web`, builds
with the public URL `/app/`, and copies `dist/web` to
`static/app/`. `npm start` runs it automatically; the docs workflow runs it
before `npm run check`.

The tracked `static/app/index.html` is a hand-written placeholder that explains
how to build the app, so `/play` degrades to that explanation instead of a bare
404 when the build is absent. `build:app` replaces it, and the other files in
`static/app/` are generated and ignored by Git; do not commit the staged build,
even though the placeholder shares its path. A plain `npm run build` without
Rust therefore produces a site whose `/play` frame shows the placeholder.

Links to `/play` and `/app/` warm the app on hover. The site fetches the
generated entry document, reads the hashed JavaScript and WASM names out of it,
and downloads them into the browser cache at low priority without starting the
app, so the frame reuses cached bytes. The extraction is covered by `npm test`
through `scripts/check-web-preload.mjs`; the listener lives in
`src/webAppPreload.ts` and is installed by `src/theme/Root.tsx`.

Override the public URL with `CALLER_WEB_PUBLIC_URL` if you deploy the site at a
different base path.

## Mobile preview

`npm run preview:ios` serves the production build in a 390×844 iPhone frame
with an iPhone Safari user agent injected before the site bundle runs. Use it to
review phone layout and code that reads the browser's platform, such as the
adaptive download button. It reads `website/build`, so run `npm run build`
first; run `npm run build:app` before that when reviewing `/play`, because the
docs build alone does not stage the WASM app.

```sh
npm run build:app
npm run build
npm run preview:ios -- --path /play/
```

Options include `--device ipad`, `--width`/`--height` for another viewport,
`--port` to avoid a busy port, and `--host 0.0.0.0` to open the preview from a
real phone on the same network. The script prints the frame URL and the page URL
inside it. It does not rebuild the site, and it warns when `/app/` looks like a
Trunk build without its WASM bundle.

## Download links

The `DownloadButton` component links straight at GitHub release assets, so a
visitor never has to find the file on GitHub. The auto-detected button is the
exception on iOS, where it links to the hosted browser app at `/play` instead
of a release file; `platform="hosted"` offers that same hosted copy on any
browser, and `variant="link"` renders a build as an inline link for pages where
downloading is an aside. Those file names carry the
version, which comes from `src/release.ts`. That line mirrors
`[workspace.package] version` in `Cargo.toml`; `scripts/release.mjs` rewrites
both in the release commit. Keep them in step when you change the version by
hand.

## Screenshot manifest

`website/screenshots/manifest.json` lists every planned shot ID with its alt
text and caption. A shot renders as an image when its `status` is `ready` and
`static/img/screens/<id>.png` exists. Until then it renders as a pending
placeholder so the prose can be reviewed without fake UI.
