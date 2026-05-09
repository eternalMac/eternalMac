pub fn run(session: Option<&str>) {
    let session = session.unwrap_or("default");
    println!(
        "{:?}",
        crate::tooling::et::build_attach_args("mac-mini", session)
    );
}
