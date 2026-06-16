pub mod app {
    pub mod context;
    pub mod paths;
}
pub mod cli;
pub mod commands;
pub mod config {
    pub mod store;
}
pub mod doctor {
    pub mod service;
}
pub mod daemon {
    pub mod client;
    pub mod server;
}
pub mod dotsync;
pub mod model {
    pub mod config;
    pub mod state;
}
pub mod platform {
    pub mod launchd;
}
pub mod process {
    pub mod runner;
}
pub mod session {
    pub mod service;
}
pub mod sync {
    pub mod service;
}
pub mod setup {
    pub mod client;
    pub mod prompts;
    pub mod server;
}
pub mod status {
    pub mod service;
}
pub mod tooling {
    pub mod brew;
    pub mod dependencies;
    pub mod et;
    pub mod mutagen;
    pub mod ping;
    pub mod ssh;
    pub mod tailscale;
    pub mod tmux;
}
