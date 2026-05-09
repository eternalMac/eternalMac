pub fn build_attach_args(server: &str, session: &str) -> Vec<String> {
    vec![
        server.to_string(),
        "-c".into(),
        format!("tmux attach -t {session}"),
    ]
}
