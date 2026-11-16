//! The `contra-mcp` MCP server: the egui UI tools plus session-tab tools.
//!
//! The generic input tools (`click`, `drag`, `press_key`, …) come from
//! [`egui_mcp::UiServer`]; this server owns the connection lifecycle and adds
//! the session tools an agent needs to test the practice screen. The session
//! tools open their own inspection connection because they read AccessKit node
//! state (`toggled`, `numeric_value`) that the generic widget projection does
//! not expose.

use std::sync::Arc;
use std::time::Duration;

use egui_mcp::{Bridge, UiServer};
use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{
        router::tool::ToolRouter,
        tool::ToolCallContext,
        wrapper::{Json, Parameters},
    },
    model::{
        CacheScope, CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock,
        Implementation, ListToolsResult, PaginatedRequestParams, ServerCapabilities, ServerConfig,
    },
    schemars::JsonSchema,
    service::{RequestContext, RoleServer},
    tool, tool_router,
};
use serde::Deserialize;
use serde_json::json;
use tokio::sync::Mutex;

use crate::session::{
    SessionAction, SessionClient, SessionSet, SessionState, SwipeDirection, TransportState,
};

/// A recoverable tool failure, rendered by `rmcp` as an error tool result.
type ToolError = String;

const NOT_CONNECTED: &str = "no app connected — call `attach` first";

#[derive(Clone)]
pub struct ContraServer {
    /// The generic egui tools' connection.
    ui: Arc<Mutex<Option<UiServer>>>,
    /// The session tools' connection.
    session: Arc<Mutex<Option<SessionClient>>>,
    /// Peer identity captured at attach; the UI server keeps its bridge private.
    peer: Arc<Mutex<Option<egui_mcp::PeerInfo>>>,
    ui_router: ToolRouter<UiServer>,
    lifecycle_router: ToolRouter<Self>,
    session_router: ToolRouter<Self>,
}

impl ContraServer {
    pub fn new() -> Self {
        Self {
            ui: Arc::new(Mutex::new(None)),
            session: Arc::new(Mutex::new(None)),
            peer: Arc::new(Mutex::new(None)),
            ui_router: UiServer::router(),
            lifecycle_router: Self::lifecycle_router(),
            session_router: Self::session_router(),
        }
    }

    fn all_tools(&self) -> Vec<rmcp::model::Tool> {
        let mut tools = self.lifecycle_router.list_all();
        tools.extend(self.session_router.list_all());
        tools.extend(self.ui_router.list_all());
        tools
    }
}

impl Default for ContraServer {
    fn default() -> Self {
        Self::new()
    }
}

fn text_error(message: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(message.into())])
}

/// The `tools/list` result, with the cache hints MCP 2026-07-28 requires.
///
/// The tool list is fixed for the life of the process and holds nothing
/// user-specific, so it may be cached and shared freely.
fn tool_list(tools: Vec<rmcp::model::Tool>) -> ListToolsResult {
    const TTL: Duration = Duration::from_mins(5);
    ListToolsResult::with_all_items(tools)
        .with_ttl_ms(u64::try_from(TTL.as_millis()).unwrap_or(u64::MAX))
        .with_cache_scope(CacheScope::Public)
}

// ---------------------------------------------------------------------------------------
// Args
// ---------------------------------------------------------------------------------------

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttachArgs {
    /// Host the app's inspection port is on. Defaults to `127.0.0.1`.
    #[serde(default = "default_host")]
    pub host: String,

    /// Port the app is listening on. Defaults to `5719`.
    #[serde(default = "default_port")]
    pub port: u16,

    /// How long to keep retrying the connection, in seconds. Defaults to 10.
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

fn default_host() -> String {
    "127.0.0.1".to_owned()
}

fn default_port() -> u16 {
    5719
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EmptyArgs {}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SessionActionArgs {
    /// `play`, `pause`, `restart`, `end`, `cancel_ending`, `undo`, or `redo`.
    ///
    /// `end` presses **End tune** (or **Cancel ending** while an ending is
    /// pending); `cancel_ending` only cancels a pending ending. `undo` and
    /// `redo` press the app's Ctrl/Cmd+Z and Ctrl/Cmd+Y chords, so they work on
    /// any screen and leave the front tab alone.
    pub action: SessionAction,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SessionWaitArgs {
    /// Wait for this transport state: `ready`, `count_in`, `playing`, `paused`,
    /// `ending`, or `finished`.
    #[serde(default)]
    pub transport: Option<TransportState>,

    /// Wait until the displayed beat is at least this value.
    #[serde(default)]
    pub min_beat: Option<f64>,

    /// Wait until the displayed beat is at most this value.
    #[serde(default)]
    pub max_beat: Option<f64>,

    /// Give up after this many seconds. Defaults to 10.
    #[serde(default = "default_wait_timeout")]
    pub timeout_secs: u64,
}

fn default_wait_timeout() -> u64 {
    10
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SwipeArgs {
    /// Which way the finger travels: `left` moves to the next tab and `right`
    /// to the previous one.
    pub direction: SwipeDirection,
}

// ---------------------------------------------------------------------------------------
// Connection lifecycle
// ---------------------------------------------------------------------------------------

#[tool_router(router = lifecycle_router)]
impl ContraServer {
    /// Connect to a running Contra Karaoke window (a build with the
    /// `inspection` feature, launched with `EGUI_INSPECTION` set).
    /// Defaults to 127.0.0.1:5719 and retries until `timeout_secs` elapses.
    /// On success the app-driving and session tools start working.
    #[tool(
        title = "Attach to app",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn attach(
        &self,
        Parameters(args): Parameters<AttachArgs>,
    ) -> Result<CallToolResult, McpError> {
        let mut guard = self.ui.lock().await;
        if guard.is_some() {
            return Ok(text_error(
                "already connected — call `disconnect` first before attaching again",
            ));
        }
        let timeout = args.timeout_secs.map(Duration::from_secs);
        let bridge = match Bridge::connect(&args.host, args.port, timeout).await {
            Ok(bridge) => bridge,
            Err(error) => return Ok(text_error(format!("attach failed: {error}"))),
        };
        let info = bridge.peer_info.clone();
        // The session tools need their own connection to read AccessKit node
        // state the generic widget projection leaves out.
        let session_bridge = match Bridge::connect(&args.host, args.port, timeout).await {
            Ok(bridge) => bridge,
            Err(error) => return Ok(text_error(format!("attach failed: {error}"))),
        };
        *guard = Some(UiServer::new(bridge));
        *self.session.lock().await = Some(SessionClient::new(session_bridge));
        *self.peer.lock().await = Some(info.clone());
        Ok(CallToolResult::structured(
            json!({ "ok": true, "attached": info }),
        ))
    }

    /// Disconnect from the attached app. Tools remain available but return an
    /// error until `attach` is called again.
    #[tool(
        title = "Disconnect",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn disconnect(&self, _p: Parameters<EmptyArgs>) -> Result<CallToolResult, McpError> {
        let connected = self.ui.lock().await.take().is_some();
        *self.session.lock().await = None;
        *self.peer.lock().await = None;
        if connected {
            Ok(CallToolResult::structured(json!({ "ok": true })))
        } else {
            Ok(text_error(NOT_CONNECTED))
        }
    }

    /// Report whether an app is connected and its peer info.
    #[tool(
        title = "Connection status",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn status(&self, _p: Parameters<EmptyArgs>) -> Result<CallToolResult, McpError> {
        let peer = self.peer.lock().await.clone();
        let body = match peer {
            None => json!({ "state": "idle" }),
            Some(peer) => json!({ "state": "connected", "peer": peer }),
        };
        Ok(CallToolResult::structured(body))
    }
}

// ---------------------------------------------------------------------------------------
// Session tab
// ---------------------------------------------------------------------------------------

#[tool_router(router = session_router)]
impl ContraServer {
    /// Read the Session tab as data: the tab in front, transport state and beat,
    /// tempo, volume, and the Loop, Cycle tunes, Count-in, and Speak calls
    /// toggles. While Preferences is open, also whether **Undo** and **Redo**
    /// are available.
    ///
    /// This reports what the session screen shows, so it can verify an action:
    /// `{"transport": "paused", "beat": 12.0, ...}`. The beat is the whole beat
    /// the progress bar displays. `undo_available` and `redo_available` are
    /// `null` unless the Interface section is on screen.
    #[tool(
        title = "Session state",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn session_state(
        &self,
        _p: Parameters<EmptyArgs>,
    ) -> Result<Json<SessionState>, ToolError> {
        let guard = self.session.lock().await;
        let client = guard.as_ref().ok_or(NOT_CONNECTED)?;
        Ok(Json(client.state().await?))
    }

    /// Press a transport button the way a user does: `play`, `pause`,
    /// `restart`, `end`, or `cancel_ending`. `undo` and `redo` press the
    /// app-wide Ctrl/Cmd+Z and Ctrl/Cmd+Y chords instead.
    ///
    /// The session tab is brought to the front first, so this works from any
    /// screen. Returns the state after the press. `play` and `pause` press the
    /// one Play/Pause toggle; asking for the state the session is already in
    /// does nothing. `play` renders pending music synchronously, so the call
    /// may take a moment; `pause` stops both audio and speech; `end` presses
    /// **End tune** (or **Cancel ending** when an ending is already pending).
    /// `undo` and `redo` do not switch tabs.
    #[tool(
        title = "Session transport",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn session_action(
        &self,
        Parameters(args): Parameters<SessionActionArgs>,
    ) -> Result<Json<SessionState>, ToolError> {
        let guard = self.session.lock().await;
        let client = guard.as_ref().ok_or(NOT_CONNECTED)?;
        Ok(Json(client.action(args.action).await?))
    }

    /// Set session controls by name: `tempo_bpm`, `volume`, and the
    /// `loop_enabled`, `cycle_tunes`, `count_in`, and `speak_calls` toggles.
    /// Omitted fields are left alone.
    ///
    /// Sliders are tapped at their handle and then dragged from it. A touch
    /// build guards a slider until it is tapped; the tap is at the current
    /// handle so it does not jump the level. egui snaps a dragged slider to a
    /// nearby convenient value and the rail is quantised to a pixel, so the
    /// applied tempo or volume can differ from the request; the returned state
    /// reports the value the app actually adopted. Returns the state after the
    /// changes.
    #[tool(
        title = "Set session controls",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn session_set(
        &self,
        Parameters(args): Parameters<SessionSet>,
    ) -> Result<Json<SessionState>, ToolError> {
        let guard = self.session.lock().await;
        let client = guard.as_ref().ok_or(NOT_CONNECTED)?;
        Ok(Json(client.apply(&args).await?))
    }

    /// Poll the session until the transport reaches a state or the displayed
    /// beat crosses a bound, or until `timeout_secs` elapses.
    ///
    /// Use this instead of sleeping: `{"min_beat": 8}` waits for the eighth beat
    /// of the loop, so a timing check can look at the call prompt at a known
    /// point in the dance. The beat is the whole beat shown on the progress bar.
    #[tool(
        title = "Wait for session state",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn session_wait(
        &self,
        Parameters(args): Parameters<SessionWaitArgs>,
    ) -> Result<Json<SessionState>, ToolError> {
        if args.transport.is_none() && args.min_beat.is_none() && args.max_beat.is_none() {
            return Err(
                "session_wait needs `transport`, `min_beat`, or `max_beat` to wait for".to_owned(),
            );
        }
        let guard = self.session.lock().await;
        let client = guard.as_ref().ok_or(NOT_CONNECTED)?;
        Ok(Json(
            client
                .wait(
                    args.transport,
                    args.min_beat,
                    args.max_beat,
                    Duration::from_secs(args.timeout_secs),
                )
                .await?,
        ))
    }

    /// Swipe across the screen with a finger to change tabs, the way the phone
    /// and tablet builds work, and report the state afterwards.
    ///
    /// A tab swipe is a touch gesture, so a mouse `drag` no longer emulates one;
    /// this sends the touch events a finger produces at a point of empty screen
    /// content. `left` moves to the next tab and `right` to the previous one, and
    /// both wrap. Detail screens (a download search, the ABC editor, the alias
    /// editor) step from the tab that contains them.
    #[tool(
        title = "Swipe tabs",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn swipe(
        &self,
        Parameters(args): Parameters<SwipeArgs>,
    ) -> Result<Json<SessionState>, ToolError> {
        let guard = self.session.lock().await;
        let client = guard.as_ref().ok_or(NOT_CONNECTED)?;
        Ok(Json(client.swipe(args.direction).await?))
    }
}

// ---------------------------------------------------------------------------------------
// MCP handler
// ---------------------------------------------------------------------------------------

const INSTRUCTIONS: &str = r#"This server drives a live Contra Karaoke window: it reads the app's accessibility tree and synthesizes real input events. Work in an observe → act → verify loop.

Getting started:
- Call `attach` first (check `status` if unsure); app-driving tools return "no app connected" until then.
- Use `widget_tree` to discover widgets and ids, `screenshot` to see the frame, and `wait_for` for async UI or animations.

Testing the session tab:
- `session_state` reports the tab in front, the transport state and beat, tempo, volume, and the Loop/Cycle tunes/Count-in/Speak calls toggles as data.
- `session_action` presses Play, Pause, Restart, End, or Cancel ending through the real buttons and returns the resulting state. `undo` and `redo` press the app's Ctrl/Cmd+Z and Ctrl/Cmd+Y chords instead, so they work on any screen and do not switch tabs.
- `session_set` taps each slider handle and then drags it, because a touch build guards a slider until it is tapped. Sliders are quantised by the rail width: read the returned value, not the requested one.
- `session_wait` polls until a transport state or a displayed beat is reached, which is how to inspect a specific point in the dance without sleeping. Play starts with a four-beat count-in, so wait for `transport: "playing"` or `min_beat: 0` before checking a call.
- `swipe` changes tabs with a finger, the way a phone or tablet does. A tab swipe is a touch gesture only, so `drag` does not emulate one.
- The session controls are located by accessible name, so both the desktop and the phone layout work; `resize` to 390×844 to test the phone layout.

Conventions:
- Everything is in logical points: raw `pos`, `resize` dimensions, `get_widget` bounds, and a default `screenshot` (`pixels_per_point: 1.0`). A widget's bounds centre is where `click` lands.
- Locators (`id`, `role`, `content_contains`, …) resolve to the widget's current position and survive layout changes; prefer them over `pos`."#;

impl ServerHandler for ContraServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("contra-mcp", env!("CARGO_PKG_VERSION")))
            .with_instructions(INSTRUCTIONS)
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        Ok(tool_list(self.all_tools()))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, McpError> {
        if self.lifecycle_router.has_route(&request.name) {
            let call = ToolCallContext::new(self, request, context);
            return self.lifecycle_router.call(call).await;
        }
        if self.session_router.has_route(&request.name) {
            let call = ToolCallContext::new(self, request, context);
            return self.session_router.call(call).await;
        }
        let guard = self.ui.lock().await;
        let Some(ui) = guard.as_ref() else {
            return Ok(text_error(NOT_CONNECTED).into());
        };
        ui.dispatch(&self.ui_router, request, context)
            .await
            .map(Into::into)
    }

    fn get_tool(&self, name: &str) -> Option<rmcp::model::Tool> {
        if let Some(tool) = self.lifecycle_router.get(name) {
            return Some(tool.clone());
        }
        if let Some(tool) = self.session_router.get(name) {
            return Some(tool.clone());
        }
        self.ui_router.get(name).cloned()
    }
}

/// Serve the tool set on stdio until the client disconnects.
///
/// # Errors
/// If the stdio transport fails or the connection ends in error.
pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    use rmcp::{ServiceExt as _, transport};

    let server = ContraServer::new();
    let running = server.serve(transport::stdio()).await?;
    let _reason = running.waiting().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_tool_sets_are_listed() {
        let server = ContraServer::new();
        let names: Vec<String> = server
            .all_tools()
            .into_iter()
            .map(|tool| tool.name.into_owned())
            .collect();
        for expected in [
            "attach",
            "disconnect",
            "status",
            "click",
            "press_key",
            "widget_tree",
            "session_state",
            "session_action",
            "session_set",
            "session_wait",
            "swipe",
        ] {
            assert!(
                names.iter().any(|name| name == expected),
                "tools/list is missing `{expected}`; got {names:?}"
            );
        }
    }

    #[test]
    fn the_tool_list_carries_both_cache_hints() {
        let result = serde_json::to_value(tool_list(ContraServer::new().all_tools())).unwrap();
        assert!(result["ttlMs"].is_u64(), "ttlMs missing: {}", result);
        assert_eq!(result["cacheScope"], "public");
    }

    #[test]
    fn the_session_tools_have_descriptions() {
        let server = ContraServer::new();
        for name in [
            "session_state",
            "session_action",
            "session_set",
            "session_wait",
            "swipe",
        ] {
            let tool = server.get_tool(name).expect("a session tool");
            assert!(
                tool.description.is_some_and(|text| text.len() > 40),
                "`{name}` needs a useful description"
            );
        }
    }

    #[test]
    fn the_session_tools_are_registered_with_handlers() {
        let server = ContraServer::new();
        assert!(server.session_router.has_route("session_state"));
        assert!(server.session_router.has_route("session_action"));
        assert!(server.session_router.has_route("session_set"));
        assert!(server.session_router.has_route("session_wait"));
        assert!(server.session_router.has_route("swipe"));
        assert!(!server.session_router.has_route("click"));
    }
}
