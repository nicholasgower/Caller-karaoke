//! `contra-mcp` — an MCP server for driving a live Contra Karaoke window.
//!
//! It speaks MCP JSON-RPC over stdio. The app-driving tools (`screenshot`,
//! `widget_tree`, `click`, `type_text`, `press_key`, …) work after `attach`
//! connects to the inspection listener that a development build starts when
//! `EGUI_INSPECTION` is set. The server is a thin wrapper around
//! [`egui_mcp::server::run`], which owns the protocol and the tool set.
//!
//! Exit codes and stderr are diagnostics only; stdout is reserved for MCP.

use egui_mcp::server;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(server::run())
}

fn init_tracing() {
    use tracing_subscriber::EnvFilter;

    // `EGUI_MCP_LOG` is kept as a fallback so existing egui-mcp tooling keeps
    // working; `CONTRA_MCP_LOG` wins when both are set.
    let filter = EnvFilter::try_from_env("CONTRA_MCP_LOG")
        .or_else(|_| EnvFilter::try_from_env("EGUI_MCP_LOG"))
        .unwrap_or_else(|_| EnvFilter::new("egui_mcp=info,warn"));
    // stderr only: stdout carries the MCP JSON-RPC stream.
    let _ignored = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .try_init();
}
