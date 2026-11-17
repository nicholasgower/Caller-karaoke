# App icon

The mark is a contra set seen from above: a call bubble with three lines of
speech above four couples, each couple joined by a hold, and dashed links
joining the two couples that make a hands-four set.

- Background `#141414`, ink `#e6e6e6`.
- 512-unit grid, 96-unit corner radius, art held inside a 64-unit side margin.
- The PNGs keep transparent corners. Store and iOS listings need the full-bleed
  square that the platform then masks itself, so do not round those.

## Files

| File | Use |
| --- | --- |
| `caller-karaoke.vectorcraft` | editable master, one 512 × 512 artboard |
| `caller-karaoke.svg` | master art; favicon and scalable Linux icon |
| `caller-karaoke-16.png` … `-512.png` | taskbar, window, and Linux `hicolor` sizes |
| `caller-karaoke-1024.png` | store and documentation listings |
| `caller-karaoke-store-1024.png` | full-bleed square for iOS and stores that apply their own mask |
| `caller-karaoke.ico` | Windows executable, installer, and shortcuts |
| `../../packaging/android/res/mipmap-*` | Android launcher icons |

## Regenerating

`caller-karaoke.vectorcraft` is the editable master: one 512 × 512 artboard,
plain JSON, with a named object per part (`tile`, `call bubble`,
`speech line 1–3`, `couple 1–4 north/south/hold`, `set link`). Open it in
VectorCraft, change the art, then export the SVG and the PNG sizes from the
same artboard and rebuild the Windows icon:

```sh
node scripts/make-icon-ico.mjs
```

The Windows build embeds `caller-karaoke.ico` through `build.rs`; the desktop
window embeds `caller-karaoke-256.png`; the web build copies the SVG as its
favicon (`crates/web/index.html`).

To scale the whole mark, scale all objects about the artboard centre so the
64-unit side margin and the 96-unit corner radius stay proportional. For the
full-bleed store copy, reuse the art with a square (radius 0) background.

## Small sizes

Below 32 px the eight heads and the set links merge into a pattern. A simplified
variant (two couples, no links, thicker holds) is still to be drawn for the
favicon and the smallest launcher sizes; see `docs/roadmap/open-items.md`.
