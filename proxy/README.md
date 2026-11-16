# Web data proxy

The web (WASM) build cannot call ContraDB, ABC Session Tunes, or The Caller's
Box directly:

- `contradb.com` sends no `Access-Control-Allow-Origin`.
- `backend.abcsessiontunes.com` only allows `https://abcsessiontunes.com`.
- `www.ibiblio.org` sends no `Access-Control-Allow-Origin` for The Caller's Box.

This directory contains a small MIT-licensed Cloudflare Worker that forwards
allowlisted requests and adds CORS headers.

## Deploy

```sh
cd proxy
npm install -g wrangler
wrangler deploy
# optional but recommended:
wrangler secret put PROXY_TOKEN
```

Copy the deployed URL (for example
`https://contra-karaoke-proxy.example.workers.dev`) into
**Preferences → Web data proxy** in the web app. If you set a token, enter the
same token there.

The app calls the worker as `PROXY_URL?url=<percent-encoded upstream>`, so the
worker can forward both GET and POST requests. It never accepts arbitrary
upstream hosts, and `www.ibiblio.org` is limited to The Caller's Box path.

Without a proxy, the web build still starts and can use bundled or previously
cached content.
