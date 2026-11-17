// MIT-licensed CORS proxy for the Contra Karaoke web build.
//
// Deploy with `wrangler deploy` from this directory. Prefer setting a
// PROXY_TOKEN secret and entering the same token in the app's Preferences.
// This worker is allowlisted to the two upstream hosts used by the app.

const ALLOWED_HOSTS = new Set([
  "contradb.com",
  "www.contradb.com",
  "backend.abcsessiontunes.com",
  "abcsessiontunes.com",
]);

function corsHeaders(extra = {}) {
  return {
    "Access-Control-Allow-Origin": "*",
    "Access-Control-Allow-Headers": "Content-Type, X-Proxy-Token",
    "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
    "Access-Control-Max-Age": "86400",
    ...extra,
  };
}

export default {
  async fetch(request, env) {
    if (request.method === "OPTIONS") {
      return new Response(null, { status: 204, headers: corsHeaders() });
    }

    const url = new URL(request.url);
    const upstream = url.searchParams.get("url");
    if (!upstream) {
      return new Response("Missing url parameter", {
        status: 400,
        headers: corsHeaders(),
      });
    }

    let target;
    try {
      target = new URL(upstream);
    } catch {
      return new Response("Invalid url parameter", {
        status: 400,
        headers: corsHeaders(),
      });
    }

    if (target.protocol !== "https:" || !ALLOWED_HOSTS.has(target.hostname)) {
      return new Response("Upstream host is not allowed", {
        status: 403,
        headers: corsHeaders(),
      });
    }

    if (env.PROXY_TOKEN && request.headers.get("X-Proxy-Token") !== env.PROXY_TOKEN) {
      return new Response("Forbidden", {
        status: 403,
        headers: corsHeaders(),
      });
    }

    const headers = new Headers();
    headers.set(
      "User-Agent",
      "ContraKaraoke/0.1 (+https://github.com/nicholasgower/Contra-karaoke)",
    );
    const contentType = request.headers.get("content-type");
    if (contentType) {
      headers.set("Content-Type", contentType);
    }
    const accept = request.headers.get("accept");
    if (accept) {
      headers.set("Accept", accept);
    }

    const init = {
      method: request.method,
      headers,
      redirect: "follow",
    };
    if (request.method !== "GET" && request.method !== "HEAD") {
      init.body = request.body;
    }

    const response = await fetch(target.toString(), init);
    const body = await response.arrayBuffer();
    return new Response(body, {
      status: response.status,
      headers: corsHeaders({
        "Content-Type":
          response.headers.get("content-type") ?? "application/octet-stream",
        "Cache-Control": "public, max-age=3600",
      }),
    });
  },
};
