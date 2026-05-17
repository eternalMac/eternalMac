pub fn build_sync_destination(user: &str, host: &str, path: &str) -> String {
    format!("{user}@{host}:{path}")
}

pub fn port_probe_args(host: &str) -> Vec<String> {
    vec![
        "-G".into(),
        "5".into(),
        "-z".into(),
        host.into(),
        "22".into(),
    ]
}
