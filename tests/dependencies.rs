use eternalmac::tooling::dependencies::{build_dependency_plan, Inspector};

#[test]
fn dependency_plan_includes_only_missing_tools() {
    let inspector = Inspector::from_installed([
        ("et", false),
        ("tmux", true),
        ("mutagen", false),
        ("tailscale", true),
    ]);

    let plan = build_dependency_plan(&inspector);
    assert_eq!(plan.packages, vec!["et", "mutagen"]);
}
