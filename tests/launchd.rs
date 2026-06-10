use std::collections::BTreeMap;

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
        environment_variables: BTreeMap::new(),
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
        environment_variables: BTreeMap::new(),
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

#[test]
fn plist_render_includes_environment_variables() {
    let xml = render(&Definition {
        label: "com.eternalmac.server".into(),
        program_arguments: vec!["/opt/homebrew/bin/eternalMac".into()],
        environment_variables: BTreeMap::from([(
            String::from("PATH"),
            String::from("/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin"),
        )]),
        run_at_load: true,
        keep_alive: true,
    });

    assert!(xml.contains("<key>EnvironmentVariables</key>"));
    assert!(xml.contains("<key>PATH</key>"));
    assert!(xml.contains(
        "<string>/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin</string>"
    ));
}

#[test]
fn plist_render_escapes_xml_in_environment_variable_keys_and_values() {
    let xml = render(&Definition {
        label: "com.eternalmac.server".into(),
        program_arguments: vec!["/opt/homebrew/bin/eternalMac".into()],
        environment_variables: BTreeMap::from([(
            String::from("EVIL&KEY<"),
            String::from("v\"<>&'one"),
        )]),
        run_at_load: true,
        keep_alive: true,
    });

    assert!(
        xml.contains("<key>EVIL&amp;KEY&lt;</key><string>v&quot;&lt;&gt;&amp;&apos;one</string>")
    );
    assert!(!xml.contains("EVIL&KEY<"));
}

#[test]
fn plist_render_neutralizes_structure_breakout_in_program_arguments() {
    // A value attempting to close the <string> element and inject its own keys
    // must be fully entity-encoded so the plist structure stays intact.
    let xml = render(&Definition {
        label: "com.eternalmac.server".into(),
        program_arguments: vec!["evil</string><key>Injected</key><string>x".into()],
        environment_variables: BTreeMap::new(),
        run_at_load: true,
        keep_alive: true,
    });

    assert!(xml.contains(
        "<string>evil&lt;/string&gt;&lt;key&gt;Injected&lt;/key&gt;&lt;string&gt;x</string>"
    ));
    assert!(!xml.contains("</string><key>Injected</key>"));
}
