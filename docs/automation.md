# Agent automation (MCP)

Contra Karaoke can expose a loopback inspection endpoint that lets an MCP
client read the accessibility tree, synthesize real input events, and capture
screenshots of the running window. This is a **development and QA tool**. It is
compiled out by default and is not part of a release artifact.

The pieces are:

- **`inspection` Cargo feature** on `contra-app` — attaches the
  `egui_inspection` plugin to the running app. It is inert unless
  `EGUI_INSPECTION` is set.
- **`contra-mcp`** — an MCP server over stdio. It exposes `attach`, `screenshot`,
  `widget_tree`, `click`, `type_text`, `press_key`, and related tools, then
  forwards them to the app over the inspection protocol.

## 1. Run an inspectable app

Start the desktop app with the feature enabled and a loopback address:

```powershell
# PowerShell (Windows)
$env:EGUI_INSPECTION = "127.0.0.1:5719"
cargo run -p contra-app --features inspection
```

```sh
# bash (Linux/macOS)
EGUI_INSPECTION=127.0.0.1:5719 cargo run -p contra-app --features inspection
```

Or use the helper script:

```powershell
scripts\inspect.ps1
```

```sh
scripts/inspect.sh
```

`EGUI_INSPECTION=1` uses the default address, `127.0.0.1:5719`. Any other
`host:port` binds that address. `0`, `false`, or an empty value leaves
inspection off. Never bind a non-loopback address: the endpoint has **no
authentication**, so anyone who can reach it can read screenshots and drive the
app.

The screenshot tool needs a rendered frame. A minimized or fully occluded window
cannot be captured; bring the window to the foreground if a screenshot times
out.

## 2. Run the MCP server

The server speaks MCP JSON-RPC over stdin/stdout and logs diagnostics to
stderr. It does not start the app itself. `contra-mcp` is a thin wrapper around
the upstream `egui-mcp` server, so a client may display the server name
`egui-mcp`.

```sh
cargo run -p contra-mcp
```

For a stable command name, install it once:

```sh
cargo install --path crates/mcp
contra-mcp
```

Register it with your MCP client. The repository includes `.mcp.json` with this
configuration, which Claude Code and other clients pick up automatically:

```json
{
  "mcpServers": {
    "contra-karaoke": {
      "command": "cargo",
      "args": ["run", "--quiet", "-p", "contra-mcp"]
    }
  }
}
```

If you installed the binary, use:

```json
{
  "mcpServers": {
    "contra-karaoke": {
      "command": "contra-mcp"
    }
  }
}
```

For Claude Code, `claude mcp add contra-karaoke -- contra-mcp` works after the
install. For Codex, add to `~/.codex/config.toml`:

```toml
[mcp_servers.contra-karaoke]
command = "contra-mcp"
args = []
```

## 3. Drive the app

Call `attach` first; the app-driving tools return “no app connected” until it
succeeds. Then work in an observe → act → verify loop:

1. `widget_tree` — list widgets, roles, labels, values, and ids.
2. `get_widget` — get full text and logical-point `bounds` for one widget.
3. `click`, `hover`, `scroll`, `drag`, `type_text`, or `press_key` — act.
4. `screenshot` — look at the result (or pass `save_path`).
5. `wait_for` — wait for async UI such as search results or a re-render.

| Tool | Purpose |
| --- | --- |
| `attach` | Connect to the app; defaults to `127.0.0.1:5719`. |
| `disconnect` | Drop the connection without stopping the app. |
| `status` | Show whether an app is attached. |
| `widget_tree` | Read the accessibility tree, filtered or rooted. |
| `get_widget` | Full details and bounds for one widget id. |
| `screenshot` | PNG of the frame; returns an image content block. |
| `click` / `hover` | Click or hover a widget or raw `{x, y}` point. |
| `type_text` | Type into the focused widget or a targeted one. |
| `press_key` | Press a key such as `Space`, `R`, or `Escape`. |
| `scroll` / `drag` | Scroll a target or drag from one target to another. |
| `resize` | Change the viewport size in logical points. |
| `wait_for` | Poll until matching widgets or frames appear. |
| `batch` | Run several actions and observations in one round trip. |

Everything uses logical points in one coordinate frame: widget `bounds`, raw
positions, `resize` dimensions, and a default `screenshot` (1 pixel per point).
A widget's bounds centre is therefore where to click.

Useful checks for this app:

- Resize to a phone viewport (`resize` with `390 × 844`, for example) and
  screenshot it. The app's minimum width is 360 points.
- `press_key` with `Space` toggles play/pause; `R` restarts; `Escape` leaves a
  picker or preferences screen (the alias editor returns to preferences).
- Number keys `1`–`4` switch tabs, and `Enter` in a search field starts the
  search or adds a custom alias.
- After selecting a dance or tune, use `wait_for` on the result text before
  asserting the session screen changed.
- The screenshot is useful for theme and layout checks; `widget_tree` is better
  for behaviour because it does not depend on rendering.

## Diagnostics

Set `CONTRA_MCP_LOG` (or the upstream `EGUI_MCP_LOG`) to a `tracing` filter:

```sh
CONTRA_MCP_LOG=egui_mcp=debug,contra_mcp=debug cargo run -p contra-mcp
```

The app logs inspection startup through `env_logger`; set `RUST_LOG` if you need
more detail.

## Troubleshooting

- **`attach` times out** — the app was not built with `--features inspection`,
  `EGUI_INSPECTION` was not set, or it is listening on another port.
- **`attach` connects but tools fail** — check `status`; an app restart drops
  the connection, so call `attach` again.
- **Screenshots time out** — the window is minimized or occluded; bring it to
  the foreground.
- **Port already in use** — another app or an earlier run owns `5719`; choose a
  different port in `EGUI_INSPECTION` and in `attach`'s `port`.
- **`widget_tree` is hard to search** — use `content_contains`, then narrow with
  `role`, `id`, or `exclude`.

## Safety

- Keep the inspection feature off in release builds. `cargo build --release -p
  contra-app` does not enable it.
- Bind inspection to loopback only. The protocol has no authentication or
  encryption.
- Do not let the MCP server inherit an untrusted environment; it can send
  keystrokes and mouse events to the attached app.
- `screenshot` with `save_path` writes a PNG wherever the MCP server process can
  write. Prefer returning the image inline unless you need a file.
