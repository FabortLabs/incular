//! The embedder data root is process-wide, so it has its own test binary.

use incular_platform::{application_data_local_directory, set_application_data_root};
use std::path::PathBuf;

#[test]
fn embedder_root_holds_each_application_and_the_first_root_wins() {
    let root = PathBuf::from("/data/user/0/dev.incular.apps.demo/files");
    assert!(set_application_data_root(root.clone()));
    assert!(!set_application_data_root(PathBuf::from("/elsewhere")));
    assert_eq!(
        application_data_local_directory("dev.incular.demo"),
        Some(root.join("dev.incular.demo"))
    );
    assert_eq!(application_data_local_directory("  "), None);
}
