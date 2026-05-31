use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};

use crate::setup::client::SyncRootInput;
use crate::tooling::ssh::build_sync_destination;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DotSyncRisk {
    SafeDefault,
    Caution,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DotSyncTarget {
    pub id: &'static str,
    pub label: &'static str,
    pub include_paths: Vec<String>,
    pub default_selected: bool,
    pub risk: DotSyncRisk,
    pub risk_note: Option<&'static str>,
    pub target_exclusions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedDotSyncTarget {
    pub target: DotSyncTarget,
    pub include_path: String,
    pub local_path: PathBuf,
}

fn target(
    id: &'static str,
    label: &'static str,
    include_paths: &[&str],
    default_selected: bool,
    risk: DotSyncRisk,
    risk_note: Option<&'static str>,
    target_exclusions: &[&str],
) -> DotSyncTarget {
    DotSyncTarget {
        id,
        label,
        include_paths: include_paths.iter().map(|path| (*path).into()).collect(),
        default_selected,
        risk,
        risk_note,
        target_exclusions: target_exclusions
            .iter()
            .map(|exclusion| (*exclusion).into())
            .collect(),
    }
}

pub fn targets() -> Vec<DotSyncTarget> {
    vec![
        target(
            "claude",
            "Claude",
            &["~/.claude"],
            true,
            DotSyncRisk::SafeDefault,
            None,
            &[".claude.json"],
        ),
        target(
            "codex",
            "Codex",
            &["~/.codex"],
            true,
            DotSyncRisk::SafeDefault,
            None,
            &[],
        ),
        target(
            "opencode",
            "OpenCode",
            &["~/.config/opencode"],
            true,
            DotSyncRisk::SafeDefault,
            None,
            &[],
        ),
        target(
            "goose",
            "Goose",
            &["~/.config/goose"],
            true,
            DotSyncRisk::SafeDefault,
            None,
            &[],
        ),
        target(
            "gemini",
            "Gemini",
            &["~/.gemini"],
            true,
            DotSyncRisk::SafeDefault,
            None,
            &[],
        ),
        target(
            "qwen",
            "Qwen",
            &["~/.qwen"],
            true,
            DotSyncRisk::SafeDefault,
            None,
            &[],
        ),
        target(
            "pi",
            "Pi",
            &["~/.pi"],
            true,
            DotSyncRisk::SafeDefault,
            None,
            &[],
        ),
        target(
            "amp",
            "Amp",
            &["~/.config/amp"],
            true,
            DotSyncRisk::SafeDefault,
            None,
            &[],
        ),
        target(
            "continue",
            "Continue",
            &["~/.continue"],
            false,
            DotSyncRisk::Caution,
            Some("May include workspace indexes or extension state; review before enabling."),
            &[],
        ),
        target(
            "aider",
            "Aider",
            &["~/.aider.conf.yml"],
            false,
            DotSyncRisk::Caution,
            Some("Single config file can reference local model or provider credentials."),
            &[],
        ),
        target(
            "cline",
            "Cline",
            &["~/.cline"],
            false,
            DotSyncRisk::Caution,
            Some("Extension state may include provider configuration; sensitive providers are excluded."),
            &["data/settings/providers.json"],
        ),
        target(
            "roo",
            "Roo",
            &["~/.roo/rules", "~/.roo/rules-*"],
            false,
            DotSyncRisk::Caution,
            Some("Rules are project-agent behavior policy and should be reviewed before syncing."),
            &[],
        ),
    ]
}

pub fn standard_exclusions() -> Vec<String> {
    [
        ".DS_Store",
        ".cache",
        "cache",
        "caches",
        "tmp",
        "temp",
        "log",
        "logs",
        "*.log",
        "telemetry",
        "*telemetry*",
        "*auth*.json",
        "*token*.json",
        "*credential*.json",
        "*credentials*.json",
        "installation_id",
        "*.sqlite-wal",
        "*.sqlite-shm",
        "*.db-wal",
        "*.db-shm",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

pub fn expand_home_path(path: &str, home: &Path) -> PathBuf {
    if path == "~" {
        return home.to_path_buf();
    }

    if let Some(rest) = path.strip_prefix("~/") {
        return home.join(rest);
    }

    PathBuf::from(path)
}

fn include_paths_for_detection(include_path: &str, home: &Path) -> Result<Vec<(String, PathBuf)>> {
    if !include_path.contains('*') {
        let local_path = expand_home_path(include_path, home);
        return Ok(local_path
            .exists()
            .then(|| vec![(include_path.to_string(), local_path)])
            .unwrap_or_default());
    }

    let Some(prefix) = include_path.strip_suffix('*') else {
        return Err(anyhow!(
            "unsupported DotSync wildcard include path `{include_path}`; only trailing `*` is supported"
        ));
    };
    if prefix.contains('*') {
        return Err(anyhow!(
            "unsupported DotSync wildcard include path `{include_path}`; only one trailing `*` is supported"
        ));
    }

    let prefix_path = expand_home_path(prefix, home);
    let Some(parent) = prefix_path.parent() else {
        return Ok(vec![]);
    };
    if !parent.exists() {
        return Ok(vec![]);
    }
    let Some(file_prefix) = prefix_path.file_name().and_then(|name| name.to_str()) else {
        return Ok(vec![]);
    };

    let mut matches = fs::read_dir(parent)
        .with_context(|| format!("reading DotSync wildcard parent {}", parent.display()))?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let file_name = entry.file_name();
            let file_name = file_name.to_str()?;
            file_name.starts_with(file_prefix).then(|| {
                let include_path = format!("{prefix}{}", &file_name[file_prefix.len()..]);
                (include_path, entry.path())
            })
        })
        .collect::<Vec<_>>();
    matches.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(matches)
}

pub fn detect_existing_targets(home: &Path) -> Result<Vec<DetectedDotSyncTarget>> {
    let mut detected = Vec::new();
    for target in targets() {
        for include_path in &target.include_paths {
            for (include_path, local_path) in include_paths_for_detection(include_path, home)? {
                detected.push(DetectedDotSyncTarget {
                    target: target.clone(),
                    include_path,
                    local_path,
                });
            }
        }
    }
    Ok(detected)
}

pub fn combined_exclusions(target: &DotSyncTarget) -> Vec<String> {
    standard_exclusions()
        .into_iter()
        .chain(target.target_exclusions.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn sync_name_for_include(target: &DotSyncTarget, include_path: &str) -> String {
    if target.include_paths.len() == 1 {
        return format!("dotsync-{}", target.id);
    }

    let suffix = include_path
        .rsplit('/')
        .next()
        .unwrap_or(include_path)
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    let suffix = if suffix.is_empty() {
        "root".into()
    } else {
        suffix
    };

    format!("dotsync-{}-{suffix}", target.id)
}

pub fn build_dotsync_root(
    target: &DotSyncTarget,
    include_path: &str,
    home: &Path,
    server_ssh_user: &str,
    server_dns: &str,
) -> SyncRootInput {
    SyncRootInput {
        name: sync_name_for_include(target, include_path),
        local: expand_home_path(include_path, home).display().to_string(),
        remote: build_sync_destination(server_ssh_user, server_dns, include_path),
        ignore_paths: combined_exclusions(target),
        kind: Some("dotsync".into()),
        label: Some(target.label.to_string()),
    }
}
