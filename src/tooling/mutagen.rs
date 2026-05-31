pub const SYNC_MODE_TWO_WAY_RESOLVED: &str = "two-way-resolved";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListedSession {
    pub name: String,
    pub identifier: Option<String>,
    pub alpha_url: Option<String>,
    pub beta_url: Option<String>,
    pub status: Option<String>,
}

pub fn build_create_args(name: &str, local: &str, remote: &str) -> Vec<String> {
    build_create_args_with_ignores(name, local, remote, &[])
}

pub fn build_create_args_with_ignores(
    name: &str,
    local: &str,
    remote: &str,
    ignore_paths: &[String],
) -> Vec<String> {
    let mut args = vec![
        "sync".into(),
        "create".into(),
        "--name".into(),
        name.into(),
        "--sync-mode".into(),
        SYNC_MODE_TWO_WAY_RESOLVED.into(),
    ];
    for ignore_path in ignore_paths {
        args.push("--ignore".into());
        args.push(ignore_path.clone());
    }
    args.push(local.into());
    args.push(remote.into());
    args
}

pub fn list_args() -> Vec<String> {
    vec!["sync".into(), "list".into()]
}

pub fn parse_list_output(output: &str) -> Vec<ListedSession> {
    #[derive(Clone, Copy)]
    enum Section {
        Alpha,
        Beta,
    }

    fn push_if_complete(sessions: &mut Vec<ListedSession>, current: &mut Option<ListedSession>) {
        if let Some(session) = current.take() {
            sessions.push(session);
        }
    }

    let mut sessions = Vec::new();
    let mut current: Option<ListedSession> = None;
    let mut section: Option<Section> = None;

    for raw_line in output.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            section = None;
            continue;
        }

        if let Some(name) = line.strip_prefix("Name:") {
            push_if_complete(&mut sessions, &mut current);
            current = Some(ListedSession {
                name: name.trim().to_string(),
                identifier: None,
                alpha_url: None,
                beta_url: None,
                status: None,
            });
            section = None;
            continue;
        }

        let Some(session) = current.as_mut() else {
            continue;
        };

        if let Some(identifier) = line.strip_prefix("Identifier:") {
            session.identifier = Some(identifier.trim().to_string());
            continue;
        }

        if let Some(status) = line.strip_prefix("Status:") {
            session.status = Some(status.trim().to_string());
            continue;
        }

        match line {
            "Alpha:" => {
                section = Some(Section::Alpha);
                continue;
            }
            "Beta:" => {
                section = Some(Section::Beta);
                continue;
            }
            _ => {}
        }

        if let Some(url) = line.strip_prefix("URL:") {
            match section {
                Some(Section::Alpha) => session.alpha_url = Some(url.trim().to_string()),
                Some(Section::Beta) => session.beta_url = Some(url.trim().to_string()),
                None => {}
            }
        }
    }

    push_if_complete(&mut sessions, &mut current);
    sessions
}
