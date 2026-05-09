use eternalmac::tooling::et::build_attach_args;

#[test]
fn attach_args_target_named_tmux_session() {
    assert_eq!(
        build_attach_args("mac-mini", "default"),
        vec!["mac-mini", "-c", "tmux attach -t 'default'"]
    );
}

#[test]
fn attach_args_quote_session_names_with_spaces() {
    assert_eq!(
        build_attach_args("mac-mini", "pair programming"),
        vec!["mac-mini", "-c", "tmux attach -t 'pair programming'"]
    );
}

#[test]
fn attach_args_escape_single_quotes_in_session_names() {
    assert_eq!(
        build_attach_args("mac-mini", "dhruvil's session"),
        vec![
            "mac-mini",
            "-c",
            "tmux attach -t 'dhruvil'\\''s session'"
        ]
    );
}
