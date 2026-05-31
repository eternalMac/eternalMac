use serde::{Deserialize, Serialize};

use crate::model::config::Role;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPairState {
    pub name: String,
    pub local: String,
    pub remote: String,
    pub mode: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ignore_paths: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub role: Role,
    pub tailscale_ok: bool,
    pub server_reachable: bool,
    pub healthy: bool,
    pub summary: String,
    pub tailscale_dns: Option<String>,
    #[serde(default)]
    pub daemon_healthy: bool,
    #[serde(default)]
    pub daemon_heartbeat_unix: i64,
    #[serde(default)]
    pub default_session_present: bool,
    #[serde(default)]
    pub known_sessions: Vec<String>,
    #[serde(default)]
    pub syncs: Vec<SyncPairState>,
}
