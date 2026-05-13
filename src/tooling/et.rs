fn quote_shell_arg(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

pub fn build_remote_command_args(server: &str, command: &str) -> Vec<String> {
    vec![server.to_string(), "-c".into(), command.to_string()]
}

pub fn build_attach_args(server: &str, session: &str) -> Vec<String> {
    build_remote_command_args(
        server,
        &format!("tmux attach -t {}", quote_shell_arg(session)),
    )
}

pub fn build_list_sessions_args(server: &str) -> Vec<String> {
    build_remote_command_args(server, "tmux list-sessions -F '#S'")
}

pub fn build_new_session_args(server: &str, session: &str) -> Vec<String> {
    build_remote_command_args(
        server,
        &format!("tmux new-session -d -s {}", quote_shell_arg(session)),
    )
}
