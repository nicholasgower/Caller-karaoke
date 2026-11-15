# Web packaging

The web build uses Trunk and the `wasm32-unknown-unknown` target. Planned
entry point:

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
trunk build --release
```

The app must be built with the WASM audio feature of `cpal` and the browser
storage/network adapters before this is a supported release artifact. The
Cloudflare Worker proxy in `proxy/` supplies CORS-safe source access.

Output will be published from `dist/` as `contra-karaoke-web.zip`.
