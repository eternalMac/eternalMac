use eternalmac::commands::sync::{add_output, list_output, status_output};
use eternalmac::sync::service::build_pair;
use eternalmac::tooling::mutagen::build_create_args;

#[test]
fn sync_pair_defaults_to_last_write_wins() {
    let pair = build_pair("project", "~/src/project", "~/remote/project");
    assert_eq!(pair.name, "project");
    assert_eq!(pair.mode, "last-write-wins");
}

#[test]
fn mutagen_create_args_include_sync_mode_in_order() {
    let args = build_create_args("project", "~/src/project", "~/remote/project");
    assert_eq!(
        args,
        vec![
            "sync",
            "create",
            "--name",
            "project",
            "--sync-mode",
            "two-way-resolved",
            "~/src/project",
            "~/remote/project",
        ]
        .into_iter()
        .map(String::from)
        .collect::<Vec<_>>()
    );
}

#[test]
fn sync_add_output_matches_printed_command() {
    assert_eq!(
        add_output("project", "~/src/project", "~/remote/project"),
        "sync project ~/src/project ~/remote/project"
    );
}

#[test]
fn sync_list_output_matches_printed_command() {
    assert_eq!(list_output(), "project");
}

#[test]
fn sync_status_output_matches_printed_command() {
    assert_eq!(status_output(), "sync healthy");
}
