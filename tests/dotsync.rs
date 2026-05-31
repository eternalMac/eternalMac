use std::collections::BTreeSet;
use std::fs;

use eternalmac::dotsync::allowlist::{
    build_dotsync_root, combined_exclusions, detect_existing_targets, expand_home_path,
    standard_exclusions, targets, DotSyncRisk,
};

#[test]
fn targets_have_unique_ids() {
    let targets = targets();
    let unique_ids = targets
        .iter()
        .map(|target| target.id)
        .collect::<BTreeSet<_>>();

    assert_eq!(unique_ids.len(), targets.len());
}

#[test]
fn targets_classify_defaults_and_caution_items() {
    let targets = targets();
    let default_ids = targets
        .iter()
        .filter(|target| target.default_selected)
        .map(|target| target.id)
        .collect::<BTreeSet<_>>();
    let caution_ids = targets
        .iter()
        .filter(|target| target.risk == DotSyncRisk::Caution)
        .map(|target| target.id)
        .collect::<BTreeSet<_>>();

    assert_eq!(
        default_ids,
        BTreeSet::from(["amp", "claude", "codex", "gemini", "goose", "opencode", "pi", "qwen"])
    );
    assert_eq!(
        caution_ids,
        BTreeSet::from(["aider", "cline", "continue", "roo"])
    );
    assert!(targets
        .iter()
        .filter(|target| target.risk == DotSyncRisk::Caution)
        .all(|target| target.risk_note.is_some()));
}

#[test]
fn standard_exclusions_include_common_noise_and_secret_patterns() {
    let exclusions = standard_exclusions();

    assert!(exclusions.contains(&".DS_Store".to_string()));
    assert!(exclusions.contains(&"cache".to_string()));
    assert!(exclusions.contains(&"tmp".to_string()));
    assert!(exclusions.contains(&"logs".to_string()));
    assert!(exclusions.contains(&"telemetry".to_string()));
    assert!(exclusions.contains(&"*token*.json".to_string()));
    assert!(exclusions.contains(&"*credential*.json".to_string()));
    assert!(exclusions.contains(&"installation_id".to_string()));
    assert!(exclusions.contains(&"*.sqlite-wal".to_string()));
    assert!(exclusions.contains(&"*.sqlite-shm".to_string()));
}

#[test]
fn expand_home_path_maps_tilde_prefixes_to_home() {
    let home = tempfile::tempdir().unwrap();

    assert_eq!(
        expand_home_path("~/.codex", home.path()),
        home.path().join(".codex")
    );
    assert_eq!(expand_home_path("~", home.path()), home.path());
    assert_eq!(
        expand_home_path("/opt/eternalmac", home.path()),
        std::path::PathBuf::from("/opt/eternalmac")
    );
}

#[test]
fn detection_returns_only_existing_allowlisted_paths_and_expands_roo_wildcards() {
    let home = tempfile::tempdir().unwrap();
    fs::create_dir_all(home.path().join(".codex")).unwrap();
    fs::create_dir_all(home.path().join(".config/opencode")).unwrap();
    fs::create_dir_all(home.path().join(".roo/rules")).unwrap();
    fs::create_dir_all(home.path().join(".roo/rules-work")).unwrap();
    fs::create_dir_all(home.path().join(".secret-agent")).unwrap();

    let detected = detect_existing_targets(home.path()).unwrap();
    let detected_paths = detected
        .iter()
        .map(|detected| detected.include_path.clone())
        .collect::<BTreeSet<_>>();

    assert_eq!(
        detected_paths,
        BTreeSet::from([
            "~/.codex".to_string(),
            "~/.config/opencode".to_string(),
            "~/.roo/rules".to_string(),
            "~/.roo/rules-work".to_string()
        ])
    );
    assert!(detected
        .iter()
        .all(|detected| detected.local_path.starts_with(home.path())));
}

#[test]
fn combined_exclusions_merges_standard_and_target_specific_patterns() {
    let claude = targets()
        .into_iter()
        .find(|target| target.id == "claude")
        .unwrap();

    let exclusions = combined_exclusions(&claude);

    assert!(exclusions.contains(&".DS_Store".to_string()));
    assert!(exclusions.contains(&".claude.json".to_string()));
    assert_eq!(
        exclusions.iter().collect::<BTreeSet<_>>().len(),
        exclusions.len()
    );
}

#[test]
fn build_root_preserves_dotsync_metadata_and_exclusions() {
    let home = tempfile::tempdir().unwrap();
    let claude = targets()
        .into_iter()
        .find(|target| target.id == "claude")
        .unwrap();

    let root = build_dotsync_root(
        &claude,
        "~/.claude",
        home.path(),
        "devuser",
        "mac-mini.example.ts.net",
    );

    assert_eq!(root.name, "dotsync-claude");
    assert_eq!(
        root.local,
        home.path().join(".claude").display().to_string()
    );
    assert_eq!(
        root.remote,
        "devuser@mac-mini.example.ts.net:~/.claude".to_string()
    );
    assert_eq!(root.kind.as_deref(), Some("dotsync"));
    assert_eq!(root.label.as_deref(), Some("Claude"));
    assert!(root.ignore_paths.contains(&".DS_Store".to_string()));
    assert!(root.ignore_paths.contains(&".claude.json".to_string()));
}

#[test]
fn build_root_uses_unique_names_for_multi_include_targets() {
    let home = tempfile::tempdir().unwrap();
    let roo = targets()
        .into_iter()
        .find(|target| target.id == "roo")
        .unwrap();

    let rules = build_dotsync_root(
        &roo,
        "~/.roo/rules",
        home.path(),
        "devuser",
        "mac-mini.example.ts.net",
    );
    let custom = build_dotsync_root(
        &roo,
        "~/.roo/rules-work",
        home.path(),
        "devuser",
        "mac-mini.example.ts.net",
    );

    assert_eq!(rules.name, "dotsync-roo-rules");
    assert_eq!(custom.name, "dotsync-roo-rules-work");
    assert_ne!(rules.name, custom.name);
}
