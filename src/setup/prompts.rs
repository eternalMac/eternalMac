use anyhow::Result;
use dialoguer::{Confirm, Input};

use crate::setup::client::SyncRootInput;

pub fn prompt_server_dns(prefilled: Option<String>) -> Result<String> {
    let mut prompt = Input::<String>::new().with_prompt("Server Tailscale DNS name");
    if let Some(prefilled) = prefilled {
        prompt = prompt.with_initial_text(prefilled);
    }
    Ok(prompt.interact_text()?)
}

pub fn suggest_remote_path(local_path: &str) -> String {
    if local_path == "~" || local_path.starts_with("~/") {
        return local_path.to_string();
    }

    if let Some(rest) = local_path.strip_prefix("/Users/") {
        let mut segments = rest.splitn(2, '/');
        let _user = segments.next();
        if let Some(relative) = segments.next() {
            if relative.is_empty() {
                return "~".into();
            }
            return format!("~/{}", relative);
        }
        return "~".into();
    }

    if local_path.starts_with('/') {
        return local_path.to_string();
    }

    let relative = local_path.strip_prefix("./").unwrap_or(local_path);
    format!("~/{}", relative)
}

pub fn prompt_sync_pair(server_dns: &str, index: usize) -> Result<SyncRootInput> {
    let name: String = Input::new()
        .with_prompt(format!("Sync root #{index} name"))
        .interact_text()?;
    let local: String = Input::new()
        .with_prompt(format!("Sync root #{index} local path"))
        .interact_text()?;
    let remote: String = Input::new()
        .with_prompt(format!("Sync root #{index} remote destination"))
        .with_initial_text(format!("{}:{}", server_dns, suggest_remote_path(&local)))
        .interact_text()?;

    Ok(SyncRootInput {
        name,
        local,
        remote,
    })
}

pub fn prompt_sync_roots(server_dns: &str) -> Result<Vec<SyncRootInput>> {
    let mut sync_roots = vec![prompt_sync_pair(server_dns, 1)?];
    while Confirm::new()
        .with_prompt("Add another sync root?")
        .default(false)
        .interact()?
    {
        sync_roots.push(prompt_sync_pair(server_dns, sync_roots.len() + 1)?);
    }

    Ok(sync_roots)
}

#[cfg(test)]
mod tests {
    use super::suggest_remote_path;

    #[test]
    fn suggests_home_relative_path_for_user_home() {
        assert_eq!(suggest_remote_path("/Users/me/project"), "~/project");
    }

    #[test]
    fn keeps_absolute_non_home_paths() {
        assert_eq!(suggest_remote_path("/opt/src/project"), "/opt/src/project");
    }

    #[test]
    fn keeps_tilde_prefixed_paths() {
        assert_eq!(
            suggest_remote_path("~/workspace/project"),
            "~/workspace/project"
        );
    }

    #[test]
    fn prefixes_relative_paths_under_home() {
        assert_eq!(suggest_remote_path("project"), "~/project");
        assert_eq!(suggest_remote_path("./project"), "~/project");
    }
}
