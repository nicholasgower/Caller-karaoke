//! End-to-end check that the `contra-mcp` binary speaks MCP over stdio.
//!
//! This does not need a running app: `initialize` and `tools/list` are served
//! before `attach`, which is exactly the handshake an MCP client performs.

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

#[test]
fn lists_app_driving_tools_over_stdio() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_contra-mcp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn contra-mcp");

    let mut stdin = child.stdin.take().expect("child stdin");
    let stdout = child.stdout.take().expect("child stdout");

    // Read on a worker so a broken server cannot hang the test forever.
    let (line_tx, line_rx) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            match line {
                Ok(line) => {
                    if line_tx.send(line).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    let handshake = [
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": { "name": "contra-mcp-test", "version": "0" }
            }
        }),
        json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }),
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        }),
    ];
    for message in handshake {
        writeln!(stdin, "{message}").expect("write MCP request");
    }
    stdin.flush().expect("flush MCP requests");

    let deadline = Instant::now() + Duration::from_secs(30);
    let mut listed = None;
    while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
        let Ok(line) = line_rx.recv_timeout(remaining) else {
            break;
        };
        let Ok(message) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if message.get("id").and_then(Value::as_u64) == Some(2) {
            listed = message
                .pointer("/result/tools")
                .and_then(Value::as_array)
                .cloned();
            break;
        }
    }

    drop(stdin);
    let _ = child.kill();
    let _ = child.wait();
    let _ = reader.join();

    let tools = listed.expect("tools/list response before the deadline");
    let names: Vec<&str> = tools
        .iter()
        .filter_map(|tool| tool.get("name").and_then(Value::as_str))
        .collect();
    for expected in [
        "attach",
        "screenshot",
        "widget_tree",
        "click",
        "type_text",
        "press_key",
    ] {
        assert!(
            names.contains(&expected),
            "tools/list is missing `{expected}`; got {names:?}"
        );
    }
}
