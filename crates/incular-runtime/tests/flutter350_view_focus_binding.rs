use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::{Color, Size, time::Instant};
use incular_platform::{
    WindowEvent, WindowFocusDirection, WindowFocusEvent, WindowId, WindowLifecycle, WindowOptions,
};
use incular_runtime::Application;
use incular_widgets::Widget;

const SIZE: Size = Size {
    width: 100.,
    height: 100.,
};

fn frame(application: &mut Application, window: WindowId) {
    application
        .run_window_frame_at(window, Constraints::tight(SIZE), Instant::now())
        .unwrap()
        .unwrap();
}

#[test]
fn platform_focus_is_forwarded_without_changing_window_or_direction() {
    let primary_log = Rc::new(RefCell::new(Vec::new()));
    let secondary_log = Rc::new(RefCell::new(Vec::new()));
    let mut app = Application::new({
        let log = primary_log.clone();
        move |context| {
            log.borrow_mut().push(context.window_focused());
            Widget::box_(SIZE, Color::WHITE)
        }
    })
    .unwrap();
    let primary = app.primary_window();
    let secondary = app
        .open_window_with(WindowOptions::default(), {
            let log = secondary_log.clone();
            move |context| {
                log.borrow_mut().push(context.window_focused());
                Widget::box_(SIZE, Color::WHITE)
            }
        })
        .unwrap()
        .id();
    for window in [primary, secondary] {
        app.handle_window_event(WindowEvent::lifecycle(window, WindowLifecycle::Unfocused));
        frame(&mut app, window);
    }
    primary_log.take();
    secondary_log.take();

    let seen = Rc::new(RefCell::new(Vec::new()));
    let _observer = app.observe_window_focus({
        let seen = seen.clone();
        move |event| seen.borrow_mut().push(event)
    });
    let focus = WindowFocusEvent {
        window_id: primary,
        focused: true,
        direction: WindowFocusDirection::Forward,
    };
    app.handle_window_event(WindowEvent::focus(focus));
    assert_eq!(seen.take(), [focus]);
    frame(&mut app, primary);
    frame(&mut app, secondary);
    assert_eq!(primary_log.take(), [true]);
    assert!(secondary_log.take().is_empty());
    assert!(app.window_diagnostics(primary).unwrap().native_focused);
    assert!(!app.window_diagnostics(secondary).unwrap().native_focused);

    let blur = WindowFocusEvent {
        focused: false,
        direction: WindowFocusDirection::Undefined,
        ..focus
    };
    app.handle_window_event(WindowEvent::lifecycle(primary, WindowLifecycle::Unfocused));
    let focus_other = WindowFocusEvent {
        window_id: secondary,
        direction: WindowFocusDirection::Backward,
        ..focus
    };
    app.handle_window_event(WindowEvent::focus(focus_other));
    assert_eq!(seen.take(), [blur, focus_other]);
    frame(&mut app, primary);
    frame(&mut app, secondary);
    assert_eq!(primary_log.take(), [false]);
    assert_eq!(secondary_log.take(), [true]);
}

#[test]
fn stale_native_handles_and_dropped_observers_do_not_receive_focus_events() {
    let mut app = Application::new(|_| Widget::box_(SIZE, Color::WHITE)).unwrap();
    let old = app
        .open_window(WindowOptions::default(), Widget::box_(SIZE, Color::WHITE))
        .unwrap()
        .id();
    assert!(app.close_window(old));
    let new = app
        .open_window(WindowOptions::default(), Widget::box_(SIZE, Color::WHITE))
        .unwrap()
        .id();
    assert_eq!(new.index(), old.index());
    assert_ne!(new.generation(), old.generation());
    let seen = Rc::new(RefCell::new(Vec::new()));
    let sub = app.observe_window_focus({
        let seen = seen.clone();
        move |event| seen.borrow_mut().push(event)
    });
    let focus = WindowFocusEvent {
        window_id: old,
        focused: true,
        direction: WindowFocusDirection::Forward,
    };
    app.handle_window_event(WindowEvent::focus(focus));
    assert!(seen.take().is_empty());
    assert!(!app.window_diagnostics(new).unwrap().native_focused);
    app.handle_window_event(WindowEvent::lifecycle(new, WindowLifecycle::Focused));
    assert_eq!(
        seen.take(),
        [WindowFocusEvent {
            window_id: new,
            focused: true,
            direction: WindowFocusDirection::Undefined,
        }]
    );
    drop(sub);
    app.handle_window_event(WindowEvent::lifecycle(new, WindowLifecycle::Unfocused));
    assert!(seen.take().is_empty());
    assert!(!app.window_diagnostics(new).unwrap().native_focused);
}
