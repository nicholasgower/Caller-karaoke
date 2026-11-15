# website

User documentation for Contra Karaoke, built with Docusaurus 3.10.

## Status

The guide is written against the 0.1.0 development build (desktop + web MVP;
Android configured but device-testing pending). In-app screenshots are rendered
by the headless `contra-headless` binary and annotated with PhotoCraft. The
remaining placeholders are the manual Windows/Android installer dialogs.

The current manifest has 78 planned shots; 69 in-app shots are ready. The nine
remaining images are Windows/Android/Linux installer dialogs, which are manual
OS captures rather than app frames.

## Commands

```sh
npm install
npm start      # http://localhost:3000/Contra-karaoke/
npm test       # screenshot manifest, uniqueness, and emphasis regions
npm run check  # npm test + production build
npm run screenshots  # headless renderer + PhotoCraft annotation
npm run annotate     # re-apply PhotoCraft to the existing raw PNGs
npm run build
npm run serve
npm run clear
```

The project base URL is `/Contra-karaoke/` for GitHub Pages. Test the built site
under that path, not only at the server root.

## Screenshot manifest

`website/screenshots/manifest.json` lists every planned shot ID with its alt
text and caption. A shot renders as an image when its `status` is `ready` and
`static/img/screens/<id>.png` exists. Until then it renders as a pending
placeholder so the prose can be reviewed without fake UI.
