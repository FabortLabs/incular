//! Portable platform view identities and commands (support-001).

use incular_platform::{
    PlatformViewCommand, PlatformViewCompositing, PlatformViewId, next_platform_view_id,
};

#[test]
fn view_ids_are_unique_and_commands_name_their_view() {
    let first = next_platform_view_id();
    let second = next_platform_view_id();
    assert!(second > first);
    let id = PlatformViewId(7);
    assert_eq!(PlatformViewCommand::Dispose { id }.id(), id);
    assert_eq!(PlatformViewCommand::ClearFocus { id }.id(), id);
    // No host composites native views until one says it does.
    assert_eq!(
        PlatformViewCompositing::default(),
        PlatformViewCompositing::Unsupported
    );
}
