use std::{cell::RefCell, path::PathBuf, rc::Rc, time::Duration};

use incular_config::{Constraints, TextDirection};
use incular_core::{
    Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase, Size, time::Instant,
};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_text::{
    SelectionChangedCause, TextEditingController, TextSelection, TextStyle,
    register_application_font_as,
};
use incular_widgets::{EditableText, RawPointerEvent, SizedBox};

#[test]
fn physical_taps_drags_and_long_press_report_their_selection_causes() {
    use PointerDeviceKind::{Mouse, Touch};
    use PointerPhase::{Down, Move, Up};
    use SelectionChangedCause::{DoubleTap, Drag, LongPress, Tap};

    const AHEM: &str = "Pinned pointer editing Ahem";
    let path = std::env::var_os("INCULAR_FLUTTER_AHEM_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../.tmp/flutter/packages/flutter_tools/static/Ahem.ttf")
        });
    if !path.is_file() {
        eprintln!("Pinned Ahem unavailable; pointer geometry not run");
        return;
    }
    register_application_font_as(AHEM, std::fs::read(path).unwrap());
    let controller = TextEditingController::with_text("alpha beta gamma");
    controller.set_selection(TextSelection::collapsed(0));
    let changes = Rc::new(RefCell::new(Vec::new()));
    let editor = EditableText::new(controller.clone())
        .autofocus(true)
        .text_direction(TextDirection::Ltr)
        .style(TextStyle::new().font_family(AHEM).font_size(14.))
        .context_menu_builder(|_, _| SizedBox::square(10.).into())
        .on_selection_changed({
            let changes = changes.clone();
            move |selection, cause| changes.borrow_mut().push((selection, cause))
        });
    let mut runtime = Runtime::new(editor.into()).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Linux);
    let constraints = Constraints::loose(Size::new(800., 100.));
    let base = Instant::now();
    runtime.run_frame_at(constraints, base).unwrap();
    assert!(runtime.focused_element().is_some());
    assert!(changes.borrow().is_empty());

    let send = |runtime: &mut Runtime, time, pointer, kind, phase, x| {
        let event = RawPointerEvent::new(
            pointer,
            if kind == Mouse { 7 } else { 8 },
            kind,
            u32::from(phase != Up),
            Offset::new(x, 7.),
            phase,
            base + Duration::from_millis(time),
        )
        .with_button(matches!(phase, Down | Up).then_some(PRIMARY_POINTER_BUTTON));
        let _ = runtime.handle_raw_pointer_event(event);
    };
    let check = |selection, cause| {
        assert_eq!(controller.selection(), selection);
        assert_eq!(changes.borrow().last(), Some(&(selection, cause)));
    };

    send(&mut runtime, 0, 1, Touch, Down, 30.);
    send(&mut runtime, 50, 1, Touch, Up, 30.);
    check(TextSelection::collapsed(2), Tap);
    send(&mut runtime, 60, 2, Touch, Down, 30.);
    send(&mut runtime, 110, 2, Touch, Up, 30.);
    check(TextSelection::new(0, 5), DoubleTap);
    let field = runtime.focused_element().unwrap();
    let menu = runtime.tree().text_field_context_menu(field).unwrap();
    assert!(
        menu.is_showing(),
        "a touch double tap opens the selection toolbar"
    );
    menu.hide();

    send(&mut runtime, 710, 3, Mouse, Down, 16.);
    send(&mut runtime, 725, 3, Mouse, Move, 48.);
    send(&mut runtime, 740, 3, Mouse, Move, 114.);
    send(&mut runtime, 750, 3, Mouse, Up, 114.);
    check(TextSelection::new(1, 8), Drag);

    send(&mut runtime, 1350, 4, Touch, Down, 100.);
    runtime
        .run_frame_at(constraints, base + Duration::from_millis(1900))
        .unwrap();
    send(&mut runtime, 1910, 4, Touch, Up, 100.);
    check(TextSelection::new(6, 10), LongPress);
}
