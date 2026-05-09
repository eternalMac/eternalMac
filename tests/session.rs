use eternalmac::session::service::pin;

#[test]
fn pin_deduplicates_existing_session() {
    let pinned = pin(vec!["default".into()], "default");
    assert_eq!(pinned, vec!["default"]);
}
