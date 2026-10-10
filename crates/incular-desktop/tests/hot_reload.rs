//! Devserver protocol handling for hot reload: only patches the build driver
//! addressed to this process and build may reach patch application.
#![cfg(feature = "hot-reload")]

use incular_desktop::hot_reload::{DevServer, patch_for_process};
use serde_json::{Value, json};

const PID: u32 = 4_242;
const BUILD: u64 = 7;

fn hot_reload_message(jump_table: Value, build: Option<u64>, pid: Option<u32>) -> String {
    json!({
        "HotReload": {
            "templates": [],
            "assets": [],
            "ms_elapsed": 12,
            "jump_table": jump_table,
            "for_build_id": build,
            "for_pid": pid,
        }
    })
    .to_string()
}

fn jump_table() -> Value {
    json!({
        "lib": "libapp-patch-1.dll",
        "map": { "140698319253600": 140698400000000_u64, "4100": 8200 },
        "aslr_reference": 4096,
        "new_base_address": 8192,
        "ifunc_count": 0,
    })
}

#[test]
fn devserver_is_absent_unless_the_driver_published_an_address() {
    assert_eq!(DevServer::from_values(None, None, None), None);
    assert_eq!(
        DevServer::from_values(Some("127.0.0.1"), None, Some("3")),
        None
    );
    assert_eq!(DevServer::from_values(None, Some("8080"), None), None);
    assert_eq!(
        DevServer::from_values(Some("not an ip"), Some("8080"), None),
        None
    );
}

#[test]
fn connection_url_identifies_the_process_to_the_devserver() {
    let server =
        DevServer::from_values(Some("127.0.0.1"), Some("8080"), Some("7")).expect("devserver");
    assert_eq!(server.build_id(), BUILD);
    assert_eq!(
        server.url(140_698_319_253_600, PID),
        "ws://127.0.0.1:8080/_dioxus?aslr_reference=140698319253600&build_id=7&pid=4242"
    );
    let unnumbered =
        DevServer::from_values(Some("127.0.0.1"), Some("8080"), None).expect("devserver");
    assert_eq!(unnumbered.build_id(), 0);
}

#[test]
fn patch_addressed_to_this_process_is_accepted() {
    let message = hot_reload_message(jump_table(), Some(BUILD), Some(PID));
    let patch = patch_for_process(&message, BUILD, PID).expect("patch");
    assert_eq!(patch.function_count(), 2);

    // Drivers that do not number builds still address the process.
    let message = hot_reload_message(jump_table(), None, Some(PID));
    assert!(patch_for_process(&message, BUILD, PID).is_some());
}

#[test]
fn patches_for_another_process_or_build_are_ignored() {
    let other_process = hot_reload_message(jump_table(), Some(BUILD), Some(PID + 1));
    assert!(patch_for_process(&other_process, BUILD, PID).is_none());
    let unaddressed = hot_reload_message(jump_table(), Some(BUILD), None);
    assert!(patch_for_process(&unaddressed, BUILD, PID).is_none());
    let other_build = hot_reload_message(jump_table(), Some(BUILD + 1), Some(PID));
    assert!(patch_for_process(&other_build, BUILD, PID).is_none());
}

#[test]
fn messages_without_a_code_patch_are_ignored() {
    let assets_only = hot_reload_message(Value::Null, Some(BUILD), Some(PID));
    assert!(patch_for_process(&assets_only, BUILD, PID).is_none());
    for message in [
        "\"HotPatchStart\"",
        "\"FullReloadStart\"",
        "\"Shutdown\"",
        "{\"SomethingNew\":{}}",
        "not json",
        "",
    ] {
        assert!(
            patch_for_process(message, BUILD, PID).is_none(),
            "{message}"
        );
    }
}
