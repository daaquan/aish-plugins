// SPDX-License-Identifier: MIT
mod git;
mod message;
mod protocol;

use protocol::{Frame, WireMessage};
use std::io::{BufRead, Write};
use std::path::PathBuf;

fn main() {
    if let Err(e) = run() {
        // Errors go to the result frame; this is a last resort.
        eprintln!("commit plugin error: {e}");
        std::process::exit(1);
    }
}

struct Host {
    stdin: std::io::Stdin,
    stdout: std::io::Stdout,
    next_id: u64,
}

impl Host {
    fn new() -> Self {
        Host { stdin: std::io::stdin(), stdout: std::io::stdout(), next_id: 100 }
    }

    fn send(&mut self, frame: &Frame) -> Result<(), String> {
        let line = serde_json::to_string(frame).map_err(|e| e.to_string())?;
        let mut out = self.stdout.lock();
        writeln!(out, "{line}").map_err(|e| e.to_string())?;
        out.flush().map_err(|e| e.to_string())
    }

    fn read_frame(&mut self) -> Result<Frame, String> {
        let mut line = String::new();
        let n = self.stdin.lock().read_line(&mut line).map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("host closed stdin".into());
        }
        serde_json::from_str(line.trim_end()).map_err(|e| e.to_string())
    }

    /// Send a request and block for its response payload.
    fn request(&mut self, op: &str, payload: serde_json::Value) -> Result<serde_json::Value, String> {
        let id = self.next_id;
        self.next_id += 1;
        self.send(&Frame::Request { id, op: op.to_string(), payload })?;
        match self.read_frame()? {
            Frame::Response { ok: true, payload: Some(p), .. } => Ok(p),
            Frame::Response { ok: false, error, .. } => {
                Err(error.map(|e| format!("{}: {}", e.code, e.message)).unwrap_or_else(|| "service error".into()))
            }
            other => Err(format!("expected response, got {other:?}")),
        }
    }
}

fn run() -> Result<(), String> {
    let mut host = Host::new();

    let (cwd, args, config) = match host.read_frame()? {
        Frame::Invoke { cwd, args, config, .. } => (PathBuf::from(cwd), args, config),
        other => return Err(format!("expected invoke, got {other:?}")),
    };

    let apply = args.iter().any(|a| a == "--apply");
    let signoff = args.iter().any(|a| a == "--signoff");
    let style = config.get("style").and_then(|v| v.as_str()).unwrap_or("conventional").to_string();
    let language = config.get("language").and_then(|v| v.as_str()).unwrap_or("en").to_string();
    let model = config.get("model").and_then(|v| v.as_str()).unwrap_or("default").to_string();

    let diff = git::staged_diff(&cwd)?;
    if diff.trim().is_empty() {
        tty_print("Nothing staged. Run `git add` first.\n");
        return finish(&mut host, 0);
    }

    let messages = message::build_messages(&style, &language, &diff);
    let payload = serde_json::json!({
        "model": model,
        "messages": messages.iter().map(|m: &WireMessage| serde_json::json!({"role": m.role, "content": m.content})).collect::<Vec<_>>(),
        "temperature": 0.2,
    });
    let resp = host.request("model.chat", payload)?;
    let raw = resp.get("content").and_then(|v| v.as_str()).unwrap_or("");
    let msg = message::postprocess(raw);
    if msg.is_empty() {
        return Err(format!("model returned empty/unusable message; not committing. raw: {raw:?}"));
    }

    tty_print(&format!("\nSuggested commit:\n\n{msg}\n\n"));

    let decision = if apply || tty_confirm("Accept? [Y/n] ") {
        git::commit(&cwd, &msg, signoff)?;
        tty_print("Committed.\n");
        "applied"
    } else {
        tty_print("Aborted.\n");
        "rejected"
    };

    let usage = resp.get("usage").cloned().unwrap_or_else(|| serde_json::json!({}));
    let _ = host.request("audit.record", serde_json::json!({
        "tool": "git.commit.message.generate",
        "provider": "",
        "model": model,
        "prompt_tokens": usage.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
        "completion_tokens": usage.get("completion_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
        "decision": decision,
    }));

    finish(&mut host, 0)
}

fn finish(host: &mut Host, exit: i64) -> Result<(), String> {
    host.send(&Frame::Result { id: 1, ok: true, payload: serde_json::json!({"exit": exit}) })
}

/// Human output goes to /dev/tty, never to protocol stdout.
fn tty_print(s: &str) {
    if let Ok(mut tty) = std::fs::OpenOptions::new().write(true).open("/dev/tty") {
        let _ = tty.write_all(s.as_bytes());
    }
}

/// Prompt on /dev/tty. Returns true on Y/empty. Non-interactive (no tty) = false.
fn tty_confirm(prompt: &str) -> bool {
    use std::io::{BufRead, BufReader};
    let Ok(tty_w) = std::fs::OpenOptions::new().write(true).open("/dev/tty") else {
        return false;
    };
    let Ok(tty_r) = std::fs::OpenOptions::new().read(true).open("/dev/tty") else {
        return false;
    };
    {
        let mut w = tty_w;
        let _ = w.write_all(prompt.as_bytes());
        let _ = w.flush();
    }
    let mut line = String::new();
    if BufReader::new(tty_r).read_line(&mut line).unwrap_or(0) == 0 {
        return false;
    }
    let a = line.trim().to_lowercase();
    a.is_empty() || a == "y" || a == "yes"
}
