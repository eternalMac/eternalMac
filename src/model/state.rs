use serde::{Deserialize, Serialize};

use crate::model::config::Role;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub role: Role,
    pub tailscale_ok: bool,
    pub server_reachable: bool,
    pub healthy: bool,
    pub summary: String,
}
