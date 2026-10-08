//! gray-bash-env — give toolchain commands a real login environment.
//!
//! Port of pi's `bash-spawn-hook.ts` (MIT), which rebuilt the bash tool with
//! a spawnHook that prepended `source ~/.profile`. As a sidecar we can't
//! replace the tool, so `tool/before` on `bash` answers
//! `{decision:"modify"}` prepending
//!
//!   `[ -f ~/.profile ] && . ~/.profile; [ -f ~/.bashrc ] && . ~/.bashrc; `
//!
//! — but only when it's likely to matter: the command's first token must
//! name a tool commonly missing from non-login shells (node, npm, cargo,
//! go, uv, pyenv — the seeded list at ~/.gray/bash-env/tools.txt, user-
//! extendable), and the command must not already source a profile or run
//! under `bash -l`/`--login`. Path tokens match by basename, so
//! `~/.cargo/bin/cargo build` counts too.
//!
//! `/env on|off|add <tool>|rm <tool>|list`. Default on. Fail open.

use std::io::{BufRead, Write};
use std::path::PathBuf;

use serde_json::{Value, json};

const PRELUDE: &str =
    "[ -f ~/.profile ] && . ~/.profile; [ -f ~/.bashrc ] && . ~/.bashrc; ";

const DEFAULT_TOOLS: &[&str] = &["node", "npm", "cargo", "go", "uv", "pyenv"];

fn manifest() -> Value {
    json!({
        "name": "bash-env",
        "version": env!("CARGO_PKG_VERSION"),
        "protocol": "2.0",
        "tools": [],
        "commands": ["/env"],
        "hooks": ["tool/before"],
    })
}

fn state_dir() -> PathBuf {
    std::env::var_os("GRAY_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".gray")))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("bash-env")
}

fn enabled() -> bool {
    match std::fs::read_to_string(state_dir().join("enabled")) {
        Ok(s) => !matches!(s.trim(), "off" | "0" | "false" | "no"),
        Err(_) => true,
    }
}

fn set_enabled(on: bool) -> std::io::Result<()> {
    let dir = state_dir();
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("enabled"), if on { "on" } else { "off" })
}

fn tools_file() -> PathBuf {
    state_dir().join("tools.txt")
}

/// Load the watched-tool list, seeding the file with defaults on first use.
fn tools() -> Vec<String> {
    let file = tools_file();
    if !file.exists() {
        let _ = std::fs::create_dir_all(state_dir());
        let _ = std::fs::write(&file, DEFAULT_TOOLS.join("\n") + "\n");
    }
    std::fs::read_to_string(&file)
        .map(|s| {
            s.lines()
                .map(str::trim)
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_else(|_| DEFAULT_TOOLS.iter().map(|s| s.to_string()).collect())
}

fn write_tools(list: &[String]) -> std::io::Result<()> {
    let dir = state_dir();
    std::fs::create_dir_all(&dir)?;
    std::fs::write(tools_file(), list.join("\n") + "\n")
}

/// The command already arranges its own environment — don't double-source.
fn already_sources(command: &str) -> bool {
    if command.contains("bash -l") || command.contains("--login") {
        return true;
    }
    command.split_whitespace().any(|t| t == "source" || t == ".")
}

/// First token's basename — `./node`, `~/.cargo/bin/cargo` both count.
fn first_tool(command: &str) -> &str {
    let first = command.split_whitespace().next().unwrap_or("");
    first.rsplit('/').next().unwrap_or(first)
}

fn tool_before(params: &Value) -> Value {
    if !enabled() || params.get("name").and_then(Value::as_str) != Some("bash") {
        return json!({"decision": "allow"});
    }
    let command = params
        .get("args")
        .and_then(|a| a.get("command"))
        .and_then(Value::as_str)
        .unwrap_or("");
    if command.is_empty() || already_sources(command) {
        return json!({"decision": "allow"});
    }
    let tool = first_tool(command);
    if !tools().iter().any(|t| t == tool) {
        return json!({"decision": "allow"});
    }
    json!({
        "decision": "modify",
        "args": { "command": format!("{PRELUDE}{command}") }
    })
}

/// `/env …` — `argv` excludes the command name.
fn run_command(argv: &[&str]) -> String {
    match argv.first().copied() {
        Some("on") => match set_enabled(true) {
            Ok(()) => "bash-env on".into(),
            Err(e) => format!("couldn't write state: {e}"),
        },
        Some("off") => match set_enabled(false) {
            Ok(()) => "bash-env off".into(),
            Err(e) => format!("couldn't write state: {e}"),
        },
        Some("add") => match argv.get(1) {
            Some(tool) => {
                let mut list = tools();
                if list.iter().any(|t| t == tool) {
                    format!("{tool} already watched")
                } else {
                    list.push(tool.to_string());
                    match write_tools(&list) {
                        Ok(()) => format!("watching {tool}"),
                        Err(e) => format!("couldn't write {}: {e}", tools_file().display()),
                    }
                }
            }
            None => "usage: /env add <tool>".into(),
        },
        Some("rm") => match argv.get(1) {
            Some(tool) => {
                let mut list = tools();
                let before = list.len();
                list.retain(|t| t != tool);
                if list.len() == before {
                    format!("{tool} wasn't watched")
                } else {
                    match write_tools(&list) {
                        Ok(()) => format!("stopped watching {tool}"),
                        Err(e) => format!("couldn't write {}: {e}", tools_file().display()),
                    }
                }
            }
            None => "usage: /env rm <tool>".into(),
        },
        Some("list") => format!("watched tools: {}", tools().join(", ")),
        _ => format!(
            "gray-bash-env {} — {}prepending profile sourcing for watched tools. \
             /env on|off|add <tool>|rm <tool>|list",
            env!("CARGO_PKG_VERSION"),
            if enabled() { "" } else { "NOT " }
        ),
    }
}

/// One request → `Some(reply)`, or `None` for notifications. The bool asks
/// the loop to exit after writing the reply.
fn handle(req: &Value) -> (Option<Value>, bool) {
    let id = req.get("id").cloned();
    let method = req.get("method").and_then(Value::as_str).unwrap_or("");
    let params = req.get("params").cloned().unwrap_or(Value::Null);
    let Some(id) = id else {
        return (None, method == "plugin/shutdown");
    };
    let result = match method {
        "plugin/manifest" => manifest(),
        "tool/before" => tool_before(&params),
        "command/run" => {
            let argv: Vec<&str> = params
                .get("argv")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(Value::as_str).collect())
                .unwrap_or_default();
            json!({ "text": run_command(&argv) })
        }
        "plugin/shutdown" => return (Some(json!({ "id": id, "result": {} })), true),
        _ => {
            let error = json!({ "code": -32601, "message": "method not found" });
            return (Some(json!({ "id": id, "error": error })), false);
        }
    };
    (Some(json!({ "id": id, "result": result })), false)
}

fn main() -> std::io::Result<()> {
    if std::env::args().nth(1).as_deref() == Some("manifest") {
        println!("{}", manifest());
        return Ok(());
    }
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = line?;
        let Ok(req) = serde_json::from_str::<Value>(&line) else { continue };
        let (reply, exit) = handle(&req);
        if let Some(reply) = reply {
            writeln!(stdout, "{reply}")?;
            stdout.flush()?;
        }
        if exit {
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(method: &str, params: Value) -> Value {
        handle(&json!({ "id": 1, "method": method, "params": params }))
            .0
            .unwrap()
    }

    fn before(command: &str) -> Value {
        call(
            "tool/before",
            json!({"name": "bash", "args": {"command": command}, "session": {"id": "s"}}),
        )
    }

    #[test]
    fn manifest_shape() {
        let m = manifest();
        assert_eq!(m["name"], "bash-env");
        assert_eq!(m["hooks"], json!(["tool/before"]));
        assert_eq!(m["commands"], json!(["/env"]));
    }

    #[test]
    fn watched_tools_get_prelude() {
        for cmd in ["npm test", "cargo build", "uv run x.py", "go build ./..."] {
            let r = before(cmd);
            assert_eq!(r["result"]["decision"], "modify", "for {cmd}");
            let new = r["result"]["args"]["command"].as_str().unwrap();
            assert!(new.starts_with(PRELUDE) && new.ends_with(cmd), "got {new}");
        }
    }

    #[test]
    fn path_invocations_match_basename() {
        assert_eq!(before("~/.cargo/bin/cargo build")["result"]["decision"], "modify");
        assert_eq!(before("./node_modules/.bin/node x.js")["result"]["decision"], "modify");
    }

    #[test]
    fn unwatched_tools_pass() {
        for cmd in ["ls -la", "echo hi", "python x.py", "pip install x", ""] {
            assert_eq!(before(cmd)["result"]["decision"], "allow", "for {cmd}");
        }
    }

    #[test]
    fn already_sourcing_passes() {
        for cmd in [
            "source ~/.profile && npm test",
            ". ~/.bashrc; npm test",
            "bash -lc 'npm test'",
            "bash --login -c 'npm test'",
        ] {
            assert_eq!(before(cmd)["result"]["decision"], "allow", "for {cmd}");
        }
    }

    #[test]
    fn non_bash_allows() {
        let r = call("tool/before", json!({"name": "write", "args": {"path": "x"}}));
        assert_eq!(r["result"]["decision"], "allow");
    }

    #[test]
    fn tools_file_is_seeded() {
        let list = tools();
        for d in DEFAULT_TOOLS {
            assert!(list.iter().any(|t| t == d), "missing default {d}");
        }
    }

    #[test]
    fn shutdown_replies_then_exits() {
        let (reply, exit) = handle(&json!({ "id": 2, "method": "plugin/shutdown" }));
        assert!(reply.is_some() && exit);
    }
}
