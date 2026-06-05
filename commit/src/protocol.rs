// SPDX-License-Identifier: MIT
//! Plugin-side copy of the aish ABI contract (must match aish core's
//! src/plugin/protocol.rs for ABI major 1).
use serde::{Deserialize, Serialize};

/// ABI major this plugin targets. The host enforces the match; kept here as the
/// contract marker so the version is visible alongside the frame definitions.
#[allow(dead_code)]
pub const ABI_MAJOR: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtoError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Frame {
    Invoke {
        id: u64,
        subcommand: String,
        #[serde(default)]
        args: Vec<String>,
        cwd: String,
        #[serde(default)]
        config: serde_json::Value,
        #[serde(default)]
        services: Vec<String>,
    },
    Request {
        id: u64,
        op: String,
        #[serde(default)]
        payload: serde_json::Value,
    },
    Response {
        id: u64,
        ok: bool,
        #[serde(default)]
        payload: Option<serde_json::Value>,
        #[serde(default)]
        error: Option<ProtoError>,
    },
    Result {
        id: u64,
        ok: bool,
        #[serde(default)]
        payload: serde_json::Value,
    },
}
