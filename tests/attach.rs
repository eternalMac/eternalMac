use eternalmac::tooling::et::build_attach_args;

#[test]
fn attach_args_target_named_tmux_session() {
    assert_eq!(
        build_attach_args("mac-mini", "default"),
        vec!["mac-mini", "-c", "tmux attach -t default"]
    );
}
