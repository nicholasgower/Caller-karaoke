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
  forwards them to the app over the inspection protocol. It also adds
  session-tab tools (`session_state`, `session_action`, `session_set`, and
  `session_wait`) for testing the practice screen.

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
stderr. It does not start the app itself. `contra-mcp` uses the upstream
`egui-mcp` tool set for generic input and adds its own session-tab tools; the
server advertises itself as `contra-mcp`.

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
   `press_key` with `ArrowLeft` or `ArrowRight` moves to the previous or next
   tab. A tab swipe is a touch gesture, so the generic `drag` does not emulate
   one; use the `swipe` tool, which sends the finger gesture itself.
   `session_state` reports the tab now in front.
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
| `press_key` | Press a key such as `Space`, `R`, or `Escape`, optionally with modifiers (`command: true` is Ctrl on Windows/Linux and Cmd on macOS). |
| `scroll` / `drag` | Scroll a target or drag from one target to another. |
| `swipe` | Swipe tabs with a finger, the way a touch screen does (`left` is the next tab). |
| `resize` | Change the viewport size in logical points. |
| `wait_for` | Poll until matching widgets or frames appear. |
| `batch` | Run several actions and observations in one round trip. |
| `session_state` | Read the Session tab's transport state, beat, tempo, band volume, toggles, and (while Preferences is open) whether Undo and Redo are available. |
| `session_action` | Press the **Play/Pause** toggle, **Restart**, **End**, or **Cancel ending**; `undo` and `redo` press the keyboard chords. |
| `session_set` | Set the tempo, band volume, and the Loop/Cycle tunes/Count-in/Speak calls toggles. Each slider is tapped at its handle before it is dragged, because a touch build guards a slider until it is tapped. The **Caller volume** slider carries an accessible name, so the generic widget tools can set it too (tap it first on a touch build). |
| `session_wait` | Wait until the transport reaches a state or the beat crosses a bound. |

Everything uses logical points in one coordinate frame: widget `bounds`, raw
positions, `resize` dimensions, and a default `screenshot` (1 pixel per point).
A widget's bounds centre is therefore where to click.

Useful checks for this app:

- Resize to a phone viewport (`resize` with `390 × 844`, for example) and
  screenshot it. The app's minimum width is 360 points.
- Inspect every screen, picker, modal, empty state, and error banner at both
  `1280 × 800` and `390 × 844`; a desktop-only pass is not enough. Layout work
  is not done until the phone widths have no clipped controls or horizontal
  overflow.
- `press_key` with `Space` toggles play/pause; `R` restarts; `Escape` leaves a
  picker or preferences screen (the alias editor returns to preferences, and the
  Dance glossary tab returns to the session).
- Glossary terms are clickable labels named with the term itself; the
  **Glossary** button beside the Dance card heading has the same accessible
  name. The fourth top-row tab has the accessible name **Dance glossary**, and
  its phone label is shortened to **Terms**. The **Filter terms** field narrows the reference list. Hover a
  linked term to read its definition tooltip beside the pointer; the phone long
  press shows the same definition in a card at the top of the screen. The
  **Show dance glossary** checkbox in Appearance turns all of it off, including
  the tab.
- Media keys are not MCP inputs: the operating system's media session sits
  outside the app's widget tree, so no tool can press a hardware key.
  `session_action` (`play`, `pause`, `restart`) drives the same app path the
  session uses; use it when changing the media integration. The Android
  session is a device feature and is not reachable from `contra-mcp`.
- Number keys `1`-`4` switch tabs, `5` opens Preferences, and `Enter` in a
  search field starts the search or adds a custom or alternative alias. Use
  `swipe` to reproduce the phone tab gesture: the app reads only a finger as a
  swipe, so a `drag` over the same content stays a mouse gesture and does not
  change tabs.
- Preferences is not a tab: click the header gear, whose accessible name is
  `Preferences`. Its accessible node reports the selected state while
  Preferences or the alias editor is in front. The **Deliveries per call**
  slider in Calling carries the accessible name `Deliveries per call`, so the
  generic slider tools can change the prepared call count (tap it first on a
  touch build).
- After selecting a dance or tune, use `wait_for` on the result text before
  asserting the session screen changed.
- The screenshot is useful for theme and layout checks; `widget_tree` is better
  for behaviour because it does not depend on rendering.

## 4. Test the session tab

The generic tools can click a button, but the session tab needs more than blind
input: play and pause sound identical to a muted machine, egui reports a
checkbox's state only in the accessibility node (which the generic widget
projection drops), and a touch build guards a slider until it is tapped. The
`session_*` tools cover those gaps.

Work in the same observe → act → verify loop:

```jsonc
// 1. Where is the session, and what is it doing?
{ "name": "session_state" }
// → {"screen":"session","transport":"ready","beat":0.0,
//    "tempo_bpm":240.0,"volume":0.8,"loop_enabled":true,
//    "count_in":true,"speak_calls":false, ...}

// 2. Press the real Play/Pause button (renders pending music first).
{ "name": "session_action", "arguments": { "action": "play" } }

// 3. Wait for a known point in the dance instead of sleeping.
{ "name": "session_wait", "arguments": { "min_beat": 8 } }
// → the state once the progress bar reads beat 8

// 4. Change controls as a user would; the result is the applied state.
{ "name": "session_set", "arguments": {
    "tempo_bpm": 280, "volume": 0.5,
    "loop_enabled": false, "count_in": true
} }

// 5. Stop and verify the explicit transport state.
{ "name": "session_action", "arguments": { "action": "pause" } }
// → {"transport":"paused","beat":12.0, ...}
```

Details worth knowing:

- `transport` is one of `ready`, `count_in`, `playing`, `paused`, `ending`, or
  `finished`. The progress bar names it in the UI as well (`Playing · beat 12 /
  64`), so a screenshot shows the same state.
- `beat` is the whole beat the progress bar displays. During the count-in it is
  `null`; use `count_in_beats_left` or wait for `transport: "playing"`.
- `session_set` taps each slider handle and then drags from it, because a touch
  build guards a slider until it is tapped. egui snaps a dragged slider to a
  nearby convenient value and the rail quantises to a pixel, so the applied
  tempo or volume can differ from the request. The returned state reports the
  value the app actually adopted; assert on that, not on the request.
- `session_action` brings the Session tab to the front first, so it also works
  after inspecting a picker. `play` and `pause` share one toggle button whose
  accessible name is the action it offers; asking for the state the session is
  already in is a no-op, not a second press. `end` presses **End tune**, or
  **Cancel ending** when an ending is waiting; the ending itself starts at the
  tune's last bar, so `transport` stays `playing` until then.
- `session_action` with `undo` or `redo` presses the app's Ctrl+Z and Ctrl+Y
  chords instead of clicking, so the front tab does not change. Open
  Preferences and read `undo_available`/`redo_available` to check the stack, or
  use the generic `press_key` with `{"key": "Z", "modifiers": {"command": true}}`
  for the same shortcut. Undoing a dance or tune selection stops the old music
  and speech and leaves the session ready to play the restored tune.
- Controls are located by accessible name, so the desktop and phone layouts both
  work: `resize` to `390 × 844` and repeat the same calls to check the compact
  layout. The generic tools still cover anything the session tools do not, such
  as opening a picker or reading the dance card text.
- The **Share this session** control exists only in the browser build. Its
  accessible name is that label and its panel exposes the link as selectable
  text, but the native MCP server drives the desktop app, where the control is
  not drawn. Link building and applying are covered by the `share` unit tests
  and by `contra-perf`; a browser opens a link from its `?set=` argument.

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
  the foreground. The session itself keeps running while the window is hidden, so
  only the pixels need a visible window.
- **Port already in use** — another app or an earlier run owns `5719`; choose a
  different port in `EGUI_INSPECTION` and in `attach`'s `port`.
- **`widget_tree` is hard to search** — use `content_contains`, then narrow with
  `role`, `id`, or `exclude`.

## Headless documentation screenshots

The docs site uses a separate headless renderer rather than the live MCP
connection. It runs the same `ContraApp` with a silent transport, in-memory
cache, stub speech, and no preference file. Run it when preparing a release,
not in every feature worktree:

```sh
cd website
npm run screenshots
```

That writes raw PNGs to `website/screenshots/raw/`, drives `photocraft-cli` to
draw the focus ring from `website/screenshots/emphasis.json`, and writes the
final images to `website/static/img/screens/`. `npm run annotate` repeats only
the PhotoCraft pass. The renderer opens no window, audio device, network client,
or system TTS. It is the deterministic path for documentation captures; MCP is
for interactive live-app inspection.

Pass `mobile` as the second argument to render the phone inspection set into
`<out>/mobile` without touching the documentation manifest:

```sh
cargo run -p contra-app --features headless --bin contra-headless -- target/mobile-shots mobile
```

## Safety

- Keep the inspection feature off in release builds. `cargo build --release -p
  contra-app` does not enable it.
- Bind inspection to loopback only. The protocol has no authentication or
  encryption.
- Do not let the MCP server inherit an untrusted environment; it can send
  keystrokes and mouse events to the attached app.
- `screenshot` with `save_path` writes a PNG wherever the MCP server process can
  write. Prefer returning the image inline unless you need a file.
