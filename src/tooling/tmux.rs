pub fn new_session_args(name: &str) -> Vec<String> {
    vec!["new-session".into(), "-d".into(), "-s".into(), name.into()]
}
