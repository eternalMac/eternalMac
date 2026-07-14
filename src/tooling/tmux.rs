pub fn new_session_args(name: &str) -> Vec<String> {
    vec!["new-session".into(), "-d".into(), "-s".into(), name.into()]
}

pub fn list_sessions_args() -> Vec<String> {
    vec!["list-sessions".into(), "-F".into(), "#S".into()]
}

pub fn parse_sessions(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

/// True when a failed `tmux list-sessions` indicates "no tmux server / no
/// sessions" rather than a real error. Used to treat the benign empty case as
/// zero sessions while still surfacing genuine failures (e.g. tmux missing).
pub fn list_sessions_failed_without_server(stderr: &str) -> bool {
    let normalized = stderr.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return false;
    }

    normalized.contains("no server running on")
        || normalized.contains("failed to connect to server")
        || (normalized.contains("error connecting to")
            && (normalized.contains("no such file or directory")
                || normalized.contains("connection refused")))
}
