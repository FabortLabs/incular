use std::{cell::Cell, rc::Rc, time::Duration};

use incular_config::Constraints;
use incular_core::{Color, Size, time::Instant};
use incular_runtime::Runtime;
use incular_scroll::ScrollController;
use incular_widgets::{
    Center, Directionality, GlobalKey, KeepAlive, LayoutBuilder, ListView, Padding, Semantics,
    SizedBox, Text, TextDirection, Widget,
};

fn matches_label(runtime: &Runtime, label: &str) -> bool {
    runtime
        .tree()
        .semantics()
        .iter()
        .any(|(_, node)| node.label.as_deref() == Some(label))
}

#[test]
fn a_parked_row_can_gain_unlaid_out_padding_and_a_new_accessibility_label() {
    let scroll = ScrollController::new().with_initial_scroll_offset(3000.);
    let kept = GlobalKey::new();
    let padding = GlobalKey::new();
    let padded_layouts = Rc::new(Cell::new(0));
    let scene = |extra_padding, label| {
        let padded_layouts = padded_layouts.clone();
        let rows = (0..11).map(move |index| -> Widget {
            if index < 10 {
                let color = if index % 2 == 0 {
                    Color::rgba(255, 0, 0, 255)
                } else {
                    Color::rgba(0, 0, 255, 255)
                };
                return incular_widgets::ColoredBox::new(
                    color,
                    SizedBox::new()
                        .height(250.)
                        .child(Text::new(format!("Item {index}"))),
                )
                .into();
            }
            let text: Widget = Semantics::new(Text::new(label)).container(true).into();
            let child = if extra_padding {
                let observed = padded_layouts.clone();
                let deferred_text = LayoutBuilder::new(move |_, _| {
                    observed.set(observed.get() + 1);
                    text.clone()
                });
                Semantics::new(Widget::from(Padding::all(20., deferred_text)).with_key(padding))
                    .container(true)
                    .into()
            } else {
                text
            };
            // Keep the row host stable like Flutter's stateful ProblemWidget;
            // only its inner Text/Padding subtree changes type.
            Widget::from(KeepAlive::new(
                true,
                SizedBox::new().width(800.).child(child),
            ))
            .with_key(kept)
        });
        let list = ListView::new(rows).controller(scroll.clone());
        Directionality::new(
            TextDirection::Ltr,
            Center::new(SizedBox::new().width(800.).height(500.).child(list)),
        )
        .into()
    };
    let mut runtime = Runtime::new(scene(false, "Foo")).unwrap();
    let view = Constraints::tight(Size::new(800., 600.));
    let mut now = Instant::now();
    let mut frame = |runtime: &mut Runtime| {
        runtime.run_frame_at(view, now).unwrap();
        now += Duration::from_millis(16);
    };
    frame(&mut runtime);
    frame(&mut runtime);
    assert!(
        matches_label(&runtime, "Foo"),
        "the lower row is initially visible"
    );
    let mounted = runtime.tree().element_with_key(&kept.into()).unwrap();

    assert!(scroll.jump_to(0.));
    frame(&mut runtime);
    assert_eq!(runtime.tree().element_with_key(&kept.into()), Some(mounted));
    assert!(!matches_label(&runtime, "Foo"));
    let root = runtime.tree().root().unwrap();

    runtime.schedule_update(root, scene(true, "Foo")).unwrap();
    frame(&mut runtime);
    let new_padding = runtime
        .tree()
        .element_with_key(&padding.into())
        .expect("new parked Padding is mounted");
    assert_eq!(
        runtime
            .tree()
            .render_id(new_padding)
            .and_then(|render| runtime.tree().render_size(render)),
        Some(Size::ZERO),
    );
    assert_eq!(padded_layouts.get(), 0);
    assert!(!matches_label(&runtime, "Foo"));

    runtime.schedule_update(root, scene(true, "Bar")).unwrap();
    frame(&mut runtime);
    assert_eq!(runtime.tree().element_with_key(&kept.into()), Some(mounted));
    assert_eq!(padded_layouts.get(), 0);
    assert!(!matches_label(&runtime, "Foo") && !matches_label(&runtime, "Bar"));

    assert!(scroll.jump_to(3000.));
    for _ in 0..180 {
        frame(&mut runtime);
        if !runtime.frame_requested() && !scroll.has_pending_settle() {
            break;
        }
    }
    assert_eq!(runtime.tree().element_with_key(&kept.into()), Some(mounted));
    assert!(padded_layouts.get() > 0);
    assert!(!matches_label(&runtime, "Foo"));
    assert!(
        matches_label(&runtime, "Bar"),
        "after settling at {}, padding {:?}, semantics {}",
        scroll.offset(),
        runtime.tree().element_bounds(new_padding),
        runtime.tree().semantics_debug_dump()
    );
}
