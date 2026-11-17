#!/usr/bin/env node
// Serve the built documentation site in an iOS device frame with an iPhone or
// iPad user agent. This is for reviewing mobile layout and client-side platform
// detection (for example the adaptive download button) without an Apple device.
//
// It serves website/build as-is; rebuild the site first. See --help.

import {existsSync, readFileSync} from 'node:fs';
import {readFile, readdir, stat} from 'node:fs/promises';
import {createServer} from 'node:http';
import {networkInterfaces} from 'node:os';
import path from 'node:path';
import process from 'node:process';
import {fileURLToPath} from 'node:url';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const websiteRoot = path.resolve(scriptDir, '..');
const buildRoot = path.join(websiteRoot, 'build');
const configPath = path.join(websiteRoot, 'docusaurus.config.ts');

const DEVICES = {
  iphone: {
    label: 'iPhone',
    width: 390,
    height: 844,
    platform: 'iPhone',
    maxTouchPoints: 5,
    userAgent:
      'Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) ' +
      'AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1',
  },
  ipad: {
    label: 'iPad',
    width: 820,
    height: 1180,
    platform: 'iPad',
    maxTouchPoints: 5,
    userAgent:
      'Mozilla/5.0 (iPad; CPU OS 17_0 like Mac OS X) ' +
      'AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1',
  },
};

const PREVIEW_PATH = '/__ios-preview/';

const MIME = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.svg': 'image/svg+xml',
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.jpeg': 'image/jpeg',
  '.gif': 'image/gif',
  '.webp': 'image/webp',
  '.ico': 'image/x-icon',
  '.woff': 'font/woff',
  '.woff2': 'font/woff2',
  '.ttf': 'font/ttf',
  '.map': 'application/json; charset=utf-8',
  '.wasm': 'application/wasm',
  '.txt': 'text/plain; charset=utf-8',
};

const HELP = `Usage: npm run preview:ios -- [options]

Serve website/build in an iOS device frame with an iOS user agent. Use it to
review mobile layout and code that reads the browser's platform, without an
Apple device.

Options:
  --device <iphone|ipad>  Device preset (default: iphone)
  --width <px>            Override the preset viewport width
  --height <px>           Override the preset viewport height
  --path <route>          Site-relative page to open (default: /)
  --port <number>         Port to listen on (default: 4173; 0 picks a free one)
  --host <address>        Bind address (default: 127.0.0.1)
  -h, --help              Show this help

Run \`npm run build\` first. For /play, also run \`npm run build:app\` before the
build so the hosted WASM app is staged.`;

function fail(message) {
  console.error(`\n${message}\n`);
  process.exit(1);
}

function parseArgs(argv) {
  const options = {
    device: 'iphone',
    width: null,
    height: null,
    route: '/',
    port: 4173,
    host: '127.0.0.1',
  };

  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    if (argument === '--help' || argument === '-h') {
      console.log(HELP);
      process.exit(0);
    }
    if (!argument.startsWith('--')) {
      fail(`Unexpected argument: ${argument}\n\n${HELP}`);
    }

    const equals = argument.indexOf('=');
    const name = equals === -1 ? argument : argument.slice(0, equals);
    let value = equals === -1 ? argv[++index] : argument.slice(equals + 1);
    if (value === undefined) fail(`Missing value for ${name}`);

    switch (name) {
      case '--device':
        if (!(value in DEVICES)) {
          fail(`Unknown device: ${value}\nChoose one of: ${Object.keys(DEVICES).join(', ')}`);
        }
        options.device = value;
        break;
      case '--width':
        options.width = Number(value);
        break;
      case '--height':
        options.height = Number(value);
        break;
      case '--path':
        options.route = value;
        break;
      case '--port':
        options.port = Number(value);
        break;
      case '--host':
        options.host = value;
        break;
      default:
        fail(`Unknown option: ${name}\n\n${HELP}`);
    }
  }

  if (!Number.isInteger(options.port) || options.port < 0 || options.port > 65535) {
    fail(`Invalid port: ${options.port}`);
  }
  if (!options.host) fail('The host cannot be empty.');
  if (!options.route.startsWith('/')) {
    fail(`The path must start with "/": ${options.route}`);
  }
  if (options.width !== null && (!Number.isInteger(options.width) || options.width < 1)) {
    fail(`Invalid width: ${options.width}`);
  }
  if (options.height !== null && (!Number.isInteger(options.height) || options.height < 1)) {
    fail(`Invalid height: ${options.height}`);
  }

  return options;
}

function readBaseUrl() {
  try {
    const config = readFileSync(configPath, 'utf8');
    const match = config.match(/\bbaseUrl\s*:\s*['"]([^'"]+)['"]/);
    const segment = (match?.[1] ?? '/').replace(/^\/+|\/+$/g, '');
    return segment ? `/${segment}/` : '/';
  } catch {
    return '/';
  }
}

function sitePathFor(baseUrl, route) {
  if (route === '/') return baseUrl;
  if (baseUrl !== '/' && route.startsWith(baseUrl)) {
    fail(`Pass a site-relative path such as /play/, not ${route}.`);
  }
  return `${baseUrl}${route.replace(/^\//, '')}`;
}

function spoofScript(device) {
  return `<script>
(() => {
  const define = (name, value) => {
    try {
      Object.defineProperty(Navigator.prototype, name, {get: () => value, configurable: true});
    } catch {
      // A browser that locks the property keeps its real value; the site still loads.
    }
  };
  define('userAgent', ${JSON.stringify(device.userAgent)});
  define('platform', ${JSON.stringify(device.platform)});
  define('maxTouchPoints', ${device.maxTouchPoints});
  define('vendor', 'Apple Computer, Inc.');
})();
</script>`;
}

function framePage({device, width, height, sitePath}) {
  const scaleHint =
    height > 1000
      ? `\n    <p class="meta">Scroll to see the whole ${device.label} viewport.</p>`
      : '';
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Caller Karaoke · ${device.label} preview</title>
<style>
  :root { color-scheme: dark; }
  * { box-sizing: border-box; }
  body {
    margin: 0;
    display: grid;
    place-items: center;
    min-height: 100vh;
    background: radial-gradient(circle at 50% 0%, #262b33 0%, #101215 60%);
    color: #c9ced6;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", system-ui, sans-serif;
  }
  .stage { display: flex; flex-direction: column; align-items: center; gap: 14px; padding: 24px 16px; }
  .device {
    position: relative;
    width: ${width + 24}px;
    padding: 24px 12px 12px;
    border-radius: ${device.label === 'iPad' ? 42 : 56}px;
    background: linear-gradient(145deg, #30353d, #171a1f 55%);
    box-shadow: 0 0 0 2px #3b414a, 0 28px 80px rgba(0, 0, 0, 0.6);
  }
  .device::before {
    content: "";
    position: absolute;
    top: 6px;
    left: 50%;
    width: ${device.label === 'iPad' ? 8 : 96}px;
    height: ${device.label === 'iPad' ? 8 : 14}px;
    transform: translateX(-50%);
    border-radius: 20px;
    background: #0a0b0d;
    z-index: 2;
    pointer-events: none;
  }
  .screen {
    width: ${width}px;
    height: ${height}px;
    border-radius: ${device.label === 'iPad' ? 32 : 44}px;
    overflow: hidden;
    background: #fff;
  }
  iframe { display: block; width: 100%; height: 100%; border: 0; }
  .meta { margin: 0; font-size: 0.82rem; letter-spacing: 0.01em; color: #9aa3af; }
  .meta strong { color: #e7ebf0; font-weight: 600; }
</style>
</head>
<body>
  <div class="stage">
    <div class="device">
      <div class="screen">
        <iframe src="${sitePath}" title="Caller Karaoke website with an ${device.label} user agent"></iframe>
      </div>
    </div>
    <p class="meta"><strong>${device.label} preview</strong> · ${width} × ${height} · iOS user agent injected</p>${scaleHint}
  </div>
</body>
</html>`;
}

function send(res, status, type, body) {
  res.writeHead(status, {
    'Content-Type': type,
    'Cache-Control': 'no-store',
    'X-Caller-Preview': 'ios',
  });
  res.end(body);
}

function injectSpoof(html, device) {
  if (!/<head[^>]*>/i.test(html)) return `${spoofScript(device)}${html}`;
  return html.replace(/<head([^>]*)>/i, `<head$1>${spoofScript(device)}`);
}

async function serveFile(res, filePath, device) {
  const extension = path.extname(filePath).toLowerCase();
  let body = await readFile(filePath);
  if (extension === '.html') {
    body = Buffer.from(injectSpoof(body.toString('utf8'), device));
  }
  send(res, 200, MIME[extension] ?? 'application/octet-stream', body);
}

async function appBundleWarning() {
  const appDir = path.join(buildRoot, 'app');
  const appIndex = path.join(appDir, 'index.html');
  if (!existsSync(appIndex)) return null;

  const indexHtml = readFileSync(appIndex, 'utf8');
  if (!/TrunkApplicationStarted|caller-web/.test(indexHtml)) return null;

  const entries = await readdir(appDir);
  if (entries.some((entry) => entry.endsWith('.wasm'))) return null;
  return (
    'The build has an /app/index.html that references a WASM app, but the ' +
    'bundle is missing. Run `npm run build:app` and then `npm run build` ' +
    'before reviewing /play.'
  );
}

const options = parseArgs(process.argv.slice(2));
const device = DEVICES[options.device];
const width = options.width ?? device.width;
const height = options.height ?? device.height;
const baseUrl = readBaseUrl();
const route = options.route;
const sitePath = sitePathFor(baseUrl, route);
const wrapperAtRoot = baseUrl !== '/';
const wrapperPath = wrapperAtRoot ? '/' : PREVIEW_PATH;

if (!existsSync(path.join(buildRoot, 'index.html'))) {
  fail(
    `No production site build found at ${buildRoot}.\n` +
      'Run `npm run build` in website/ first.\n' +
      'For /play, run `npm run build:app` before that to stage the WASM app.',
  );
}

const warning = await appBundleWarning();
if (warning) console.warn(`\nWarning: ${warning}`);

function resolveSiteFile(pathname) {
  const relative = decodeURIComponent(pathname.slice(baseUrl.length));
  let filePath = path.resolve(buildRoot, relative);
  if (filePath !== buildRoot && !filePath.startsWith(`${buildRoot}${path.sep}`)) {
    return null;
  }
  if (relative.endsWith('/')) filePath = path.join(filePath, 'index.html');
  return filePath;
}

const server = createServer(async (req, res) => {
  try {
    const url = new URL(req.url ?? '/', `http://${req.headers.host ?? 'localhost'}`);
    const pathname = decodeURIComponent(url.pathname);

    if (pathname === wrapperPath || pathname === PREVIEW_PATH) {
      send(
        res,
        200,
        MIME['.html'],
        framePage({device, width, height, sitePath}),
      );
      return;
    }

    if (wrapperAtRoot && pathname === '/') {
      send(res, 200, MIME['.html'], framePage({device, width, height, sitePath}));
      return;
    }

    if (baseUrl !== '/' && pathname === baseUrl.replace(/\/$/, '')) {
      res.writeHead(308, {Location: baseUrl});
      res.end();
      return;
    }

    if (!pathname.startsWith(baseUrl)) {
      send(res, 404, 'text/plain; charset=utf-8', 'Not found');
      return;
    }

    let filePath = resolveSiteFile(pathname);
    if (!filePath) {
      send(res, 403, 'text/plain; charset=utf-8', 'Forbidden');
      return;
    }

    try {
      const info = await stat(filePath);
      if (info.isDirectory()) filePath = path.join(filePath, 'index.html');
    } catch {
      if (!path.extname(filePath)) filePath = path.join(filePath, 'index.html');
    }

    await serveFile(res, filePath, device);
  } catch {
    send(res, 404, 'text/plain; charset=utf-8', 'Not found');
  }
});

server.on('error', (error) => {
  if (error.code === 'EADDRINUSE') {
    fail(`Port ${options.port} is already in use. Try --port ${options.port + 1}.`);
  }
  console.error(error);
  process.exit(1);
});

server.listen(options.port, options.host, () => {
  const port = server.address().port;
  const previewUrl = `http://${options.host}:${port}${wrapperPath}`;
  const pageUrl = `http://${options.host}:${port}${sitePath}`;
  console.log(`\n${device.label} preview: ${previewUrl}`);
  console.log(`Page inside the frame: ${pageUrl}`);
  console.log(`Viewport: ${width} × ${height}   Base URL: ${baseUrl}`);
  if (options.host === '0.0.0.0') {
    const addresses = Object.values(networkInterfaces())
      .flat()
      .filter((entry) => entry && entry.family === 'IPv4' && !entry.internal)
      .map((entry) => `http://${entry.address}:${port}${wrapperPath}`);
    if (addresses.length > 0) {
      console.log(`\nOpen from a phone on the same network:\n  ${addresses.join('\n  ')}`);
    }
  }
  console.log('\nPress Ctrl+C to stop.');
});
