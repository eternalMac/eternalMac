pub const SESSION_LIST_BEGIN_MARKER: &str = "__ETERNALMAC_SESSIONS_BEGIN__";
pub const SESSION_LIST_END_MARKER: &str = "__ETERNALMAC_SESSIONS_END__";

fn quote_shell_arg(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

pub fn build_remote_command_args(server: &str, command: &str) -> Vec<String> {
    build_remote_command_args_with_options(server, None, None, command)
}

pub fn build_remote_command_args_with_options(
    server: &str,
    _server_ssh_user: Option<&str>,
    server_etterminal_path: Option<&str>,
    command: &str,
) -> Vec<String> {
    // ET reads the managed SSH config for the server user; keep the host argument stable.
    let mut args = vec![server.to_string()];
    if let Some(path) = server_etterminal_path.filter(|path| !path.trim().is_empty()) {
        args.push("--terminal-path".into());
        args.push(path.trim().to_string());
    }
    args.extend(["-c".into(), command.to_string()]);
    args
}

pub fn build_attach_args(server: &str, session: &str) -> Vec<String> {
    build_attach_args_with_options(server, None, None, session)
}

pub fn build_attach_args_with_options(
    server: &str,
    server_ssh_user: Option<&str>,
    server_etterminal_path: Option<&str>,
    session: &str,
) -> Vec<String> {
    build_remote_command_args_with_options(
        server,
        server_ssh_user,
        server_etterminal_path,
        &format!("tmux attach -t {}", quote_shell_arg(session)),
    )
}

pub fn build_list_sessions_args(server: &str) -> Vec<String> {
    build_list_sessions_args_with_options(server, None, None)
}

pub fn build_list_sessions_args_with_options(
    server: &str,
    server_ssh_user: Option<&str>,
    server_etterminal_path: Option<&str>,
) -> Vec<String> {
    let command = format!(
        "printf '{}\\n'; tmux list-sessions -F '#S'; printf '{}\\n'; exit",
        SESSION_LIST_BEGIN_MARKER, SESSION_LIST_END_MARKER
    );
    build_remote_command_args_with_options(
        server,
        server_ssh_user,
        server_etterminal_path,
        &command,
    )
}

pub fn build_new_session_args(server: &str, session: &str) -> Vec<String> {
    build_new_session_args_with_options(server, None, None, session)
}

pub fn build_new_session_args_with_options(
    server: &str,
    server_ssh_user: Option<&str>,
    server_etterminal_path: Option<&str>,
    session: &str,
) -> Vec<String> {
    build_remote_command_args_with_options(
        server,
        server_ssh_user,
        server_etterminal_path,
        &format!("tmux new-session -d -s {}", quote_shell_arg(session)),
    )
}
