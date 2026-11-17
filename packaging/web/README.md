# Web packaging

The web build uses Trunk and the `wasm32-unknown-unknown` target.

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
cd crates/web && trunk build --release
```

The entry point is `crates/web/src/lib.rs`; Trunk boots the app on the
`#caller-canvas` element. Output is written to `dist/web`. Package it as
`caller-karaoke-0.1.0-web.zip` for a release.

The Cloudflare Worker proxy in `proxy/` supplies CORS-safe source access; set
its URL under Preferences. Prefs and source responses are stored in
`localStorage`. Audio starts after the Play gesture, as required by browsers.
