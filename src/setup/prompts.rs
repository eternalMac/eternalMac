use anyhow::Result;
use dialoguer::{Confirm, Input};

use crate::dotsync::allowlist::{
    build_dotsync_root, detect_existing_targets, DetectedDotSyncTarget, DotSyncRisk,
};
use crate::setup::client::SyncRootInput;
use crate::tooling::ssh::{build_sync_destination, validate_ssh_host, validate_ssh_user};

fn validate_non_empty_trimmed(value: &str, field_label: &str) -> std::result::Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{field_label} cannot be empty"));
    }
    Ok(())
}

pub fn prompt_server_dns(prefilled: Option<String>) -> Result<String> {
    let mut prompt = Input::<String>::new()
        .with_prompt("Server Tailscale DNS name")
        .validate_with(|input: &String| {
            validate_non_empty_trimmed(input, "Server DNS")?;
            validate_ssh_host(input)
        });
    if let Some(prefilled) = prefilled {
        prompt = prompt.with_initial_text(prefilled);
    }
    Ok(prompt.interact_text()?.trim().to_string())
}

pub fn prompt_server_ssh_user(prefilled: Option<String>) -> Result<String> {
    let mut prompt = Input::<String>::new()
        .with_prompt("Server SSH username")
        .validate_with(|input: &String| {
            validate_non_empty_trimmed(input, "Server SSH username")?;
            validate_ssh_user(input)
        });
    if let Some(prefilled) = prefilled {
        prompt = prompt.with_initial_text(prefilled);
    }
    Ok(prompt.interact_text()?.trim().to_string())
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

pub fn prompt_sync_pair(
    server_ssh_user: &str,
    server_dns: &str,
    index: usize,
) -> Result<SyncRootInput> {
    let name: String = Input::new()
        .with_prompt(format!("Sync root #{index} name"))
        .validate_with(|input: &String| validate_non_empty_trimmed(input, "Sync name"))
        .interact_text()?;
    let name = name.trim().to_string();

    let local: String = Input::new()
        .with_prompt(format!("Sync root #{index} local path"))
        .validate_with(|input: &String| validate_non_empty_trimmed(input, "Local path"))
        .interact_text()?;
    let local = local.trim().to_string();

    let remote_path: String = Input::new()
        .with_prompt(format!("Sync root #{index} remote path on server"))
        .with_initial_text(suggest_remote_path(&local))
        .validate_with(|input: &String| validate_non_empty_trimmed(input, "Remote path"))
        .interact_text()?;
    let remote_path = remote_path.trim().to_string();
    let remote = build_sync_destination(server_ssh_user, server_dns, &remote_path);

    Ok(SyncRootInput {
        name,
        local,
        remote,
        ignore_paths: vec![],
        kind: None,
        label: None,
    })
}

pub fn prompt_sync_roots(server_ssh_user: &str, server_dns: &str) -> Result<Vec<SyncRootInput>> {
    let mut sync_roots = vec![prompt_sync_pair(server_ssh_user, server_dns, 1)?];
    while Confirm::new()
        .with_prompt("Add another sync root?")
        .default(false)
        .interact()?
    {
        sync_roots.push(prompt_sync_pair(
            server_ssh_user,
            server_dns,
            sync_roots.len() + 1,
        )?);
    }

    Ok(sync_roots)
}

pub fn prompt_enable_dotsync() -> Result<bool> {
    Ok(Confirm::new()
        .with_prompt("Enable DotSync for AI-agent dotfiles?")
        .default(false)
        .interact()?)
}

fn prompt_dotsync_target(target: &DetectedDotSyncTarget) -> Result<bool> {
    let mut prompt = format!("DotSync {} ({})?", target.target.label, target.include_path);
    if target.target.risk == DotSyncRisk::Caution {
        if let Some(risk_note) = target.target.risk_note {
            prompt.push_str(&format!(" - {risk_note}"));
        }
    }

    Ok(Confirm::new()
        .with_prompt(prompt)
        .default(target.target.default_selected)
        .interact()?)
}

pub fn prompt_dotsync_roots(server_ssh_user: &str, server_dns: &str) -> Result<Vec<SyncRootInput>> {
    if !prompt_enable_dotsync()? {
        return Ok(vec![]);
    }

    let home = std::env::var("HOME")?;
    let home = std::path::Path::new(&home);
    let detected = detect_existing_targets(home)?;
    if detected.is_empty() {
        println!("No supported AI-agent dotfiles found for DotSync.");
        return Ok(vec![]);
    }

    let mut roots = Vec::new();
    for detected_target in detected {
        if prompt_dotsync_target(&detected_target)? {
            roots.push(build_dotsync_root(
                &detected_target.target,
                &detected_target.include_path,
                home,
                server_ssh_user,
                server_dns,
            ));
        }
    }

    Ok(roots)
}

#[cfg(test)]
mod tests {
    use super::{suggest_remote_path, validate_non_empty_trimmed};
    use crate::tooling::ssh::build_sync_destination;

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

    #[test]
    fn rejects_blank_required_prompt_values() {
        assert!(validate_non_empty_trimmed("   ", "Sync name").is_err());
    }

    #[test]
    fn accepts_non_blank_required_prompt_values() {
        assert!(validate_non_empty_trimmed("project", "Sync name").is_ok());
    }

    #[test]
    fn remote_destination_is_built_from_user_host_and_path() {
        assert_eq!(
            build_sync_destination("devuser", "mac-mini.example.ts.net", "~/project"),
            "devuser@mac-mini.example.ts.net:~/project"
        );
    }
}
