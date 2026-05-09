use eternalmac::platform::launchd::{render, Definition};

#[test]
fn plist_render_includes_label_and_program() {
    let xml = render(&Definition {
        label: "com.eternalmac.server".into(),
        program_arguments: vec![
            "/opt/homebrew/bin/eternalMac".into(),
            "daemon".into(),
            "server".into(),
        ],
        run_at_load: true,
        keep_alive: true,
    });

    assert!(xml.contains("com.eternalmac.server"));
    assert!(xml.contains("<string>daemon</string>"));
}

#[test]
fn plist_render_escapes_xml_in_label_and_program_arguments() {
    let xml = render(&Definition {
        label: "com.eternalmac.server&<>'\"".into(),
        program_arguments: vec!["arg&one".into(), "arg<two>".into(), "arg'three'\"".into()],
        run_at_load: true,
        keep_alive: false,
    });

    assert!(xml.contains(
        "<key>Label</key><string>com.eternalmac.server&amp;&lt;&gt;&apos;&quot;</string>"
    ));
    assert!(xml.contains("<string>arg&amp;one</string>"));
    assert!(xml.contains("<string>arg&lt;two&gt;</string>"));
    assert!(xml.contains("<string>arg&apos;three&apos;&quot;</string>"));

    assert!(!xml.contains("com.eternalmac.server&<>'\""));
    assert!(!xml.contains("<string>arg&one</string>"));
    assert!(!xml.contains("<string>arg<two></string>"));
}
