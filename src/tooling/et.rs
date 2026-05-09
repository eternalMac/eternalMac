pub fn build_attach_args(server: &str, session: &str) -> Vec<String> {
    let quoted_session = format!("'{}'", session.replace('\'', "'\\''"));

    vec![
        server.to_string(),
        "-c".into(),
        format!("tmux attach -t {quoted_session}"),
    ]
}
