pub fn alive_check_args(host: &str) -> Vec<String> {
    // macOS ping: one probe, give up after 3 seconds overall.
    vec![
        "-c".into(),
        "1".into(),
        "-t".into(),
        "3".into(),
        host.into(),
    ]
}
