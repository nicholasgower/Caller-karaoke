# caller-sources

Provider traits and parsers for the dance and tune sites that
[Caller Karaoke][repo] uses: [ContraDB][contradb], [The Caller's Box][callersbox],
and [ABC Session Tunes][abc].

The crate builds requests and parses responses. It deliberately has no HTTP
client and no cache backend of its own: the host supplies both, so the same
providers work on desktop, Android, and in the browser.

[repo]: https://github.com/nicholasgower/Caller-karaoke
[contradb]: https://contradb.com
[callersbox]: https://www.ibiblio.org/contradance/thecallersbox/
[abc]: https://abc.sourceforge.net/abc-session-tunes/

## What is here

- `DanceProvider` / `TuneProvider` — search, detail, and URL parsing for one
  site, with no site-specific code leaking into the caller.
- `dance_providers()` / `tune_providers()` — the built-in provider registry;
  `dance_provider(id)` and `tune_provider(id)` look one up by name.
- `HttpRequest` — method, URL, headers, and body for the host's HTTP client.
- `FilterSpec` — provider-defined search filters, so a UI can render a search
  form without knowing a provider's API.
- `Cache` with `CacheStore` — a TTL cache in front of the parsed or raw body,
  with `MemoryCache` and `FileCache` backends.

## Example

```rust
use caller_sources::{DanceQuery, dance_providers};

for provider in dance_providers() {
    let request = provider.search(&DanceQuery::default());
    println!("{} -> {}", provider.id(), request.url);
}
```

Caching the raw response is the caller's job, but the crate supplies the store:

```rust
use caller_sources::{Cache, FileCache};

let mut cache = Cache::new(FileCache::new("cache/sources")?);
if let Some(body) = cache.get("contradb:dance/2409", 30 * 24 * 60 * 60) {
    // Parse the cached body instead of fetching it again.
} else {
    // Fetch through your HTTP client, then cache.put("contradb:dance/2409", &body)?;
}
```

## Web builds

Browsers block cross-origin requests to these sites. Caller Karaoke deploys a
small allowlisted Cloudflare Worker (`proxy/` in the repository) for the web
build; a browser host needs a proxy of its own, or must supply pre-fetched
bodies. The crate never makes an open-proxy friendly assumption about the host.

## Building

Part of the Caller Karaoke workspace; build and test it from the repository root:

```sh
cargo test -p caller-sources
```

## License

MIT. See [LICENSE](https://github.com/nicholasgower/Caller-karaoke/blob/main/LICENSE).
