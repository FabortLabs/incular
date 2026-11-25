//! Platform-specific Flutter WidgetsApp scroll shortcuts through the real runtime.

use std::time::Duration;

use incular_config::{Constraints, TextDirection};
use incular_core::{
    Code, KeyboardEvent, KeyboardKey, Modifiers, NamedKey, Offset, Rect, Size, time::Instant,
};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_widgets::{
    CustomScrollView, Directionality, Focus, SizedBox, Sliver, SliverToBoxAdapter, Widget,
    internal::{Key, ScrollController},
};

#[test]
fn apple_uses_command_and_linux_uses_control_for_line_scrolling() {
    use TextShortcutPlatform::{Ios, Linux, MacOs};

    for platform in [Ios, MacOs, Linux] {
        let first = Key::from(0_u64);
        let slivers = (0_u64..20).map(|index| {
            let child = Widget::from(SizedBox::new().height(50.)).with_key(Key::from(index));
            Box::new(SliverToBoxAdapter::new(
                Focus::new(child).autofocus(index == 0),
            )) as Box<dyn Sliver>
        });
        let controller = ScrollController::new();
        let mut runtime = Runtime::new(
            Directionality::new(
                TextDirection::Ltr,
                CustomScrollView::new(slivers).controller(controller.clone()),
            )
            .into(),
        )
        .unwrap();
        runtime.set_text_shortcut_platform(platform);
        let viewport = Constraints::tight(Size::new(800., 600.));
        let mut time = Instant::now();
        runtime.run_frame_at(viewport, time).unwrap();
        time += Duration::from_millis(16);
        runtime.run_frame_at(viewport, time).unwrap();
        assert!(runtime.focused_element().is_some());
        let child = runtime.tree().element_with_key(&first).unwrap();
        let check = |runtime: &Runtime, offset: f32| {
            assert_eq!(controller.offset(), offset, "{platform:?}");
            assert_eq!(
                runtime.tree().element_bounds(child).unwrap(),
                Rect::from_origin_size(Offset::new(0., -offset), Size::new(800., 50.))
            );
        };
        check(&runtime, 0.);
        let control = (NamedKey::Control, Code::ControlLeft, Modifiers::CONTROL);
        let command = (NamedKey::Meta, Code::MetaLeft, Modifiers::META);
        let (active, inactive) = if platform == Linux {
            (control, command)
        } else {
            (command, control)
        };
        for (name, code, modifier, offset) in [
            (NamedKey::ArrowDown, Code::ArrowDown, Some(inactive), 0.),
            (NamedKey::ArrowDown, Code::ArrowDown, Some(active), 50.),
            (NamedKey::ArrowUp, Code::ArrowUp, Some(active), 0.),
            (NamedKey::PageDown, Code::PageDown, None, 400.),
            (NamedKey::PageUp, Code::PageUp, None, 0.),
        ] {
            let modifiers = modifier.map_or(Modifiers::empty(), |(_, _, flags)| flags);
            if let Some((name, code, flags)) = modifier {
                let mut down = KeyboardEvent::key_down(KeyboardKey::Named(name), code);
                down.modifiers = flags;
                let _ = runtime.dispatch_keyboard_event(down);
            }
            let mut down = KeyboardEvent::key_down(KeyboardKey::Named(name), code);
            let mut up = KeyboardEvent::key_up(KeyboardKey::Named(name), code);
            down.modifiers = modifiers;
            up.modifiers = modifiers;
            let _ = runtime.dispatch_keyboard_event(down);
            let _ = runtime.dispatch_keyboard_event(up);
            if let Some((name, code, _)) = modifier {
                let _ = runtime
                    .dispatch_keyboard_event(KeyboardEvent::key_up(KeyboardKey::Named(name), code));
            }
            for _ in 0..10 {
                time += Duration::from_millis(16);
                runtime.run_frame_at(viewport, time).unwrap();
            }
            check(&runtime, offset);
        }
    }
}
