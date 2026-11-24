//! Frame-callback regression from Flutter 3.50's SchedulerBinding.
//! See `audits/flutter-3.50.0-parity/behavior-probes.md` (B06).

use incular_animation::{cancel_frame_callback, schedule_frame_callback};
use incular_config::Constraints;
use incular_core::{Color, Size, time::Instant};
use incular_runtime::Runtime;
use incular_widgets::Widget;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[test]
fn transient_callback_can_cancel_a_later_callback_in_the_same_batch() {
    let size = Size::new(100., 100.);
    let bounds = Constraints::tight(size);
    let mut runtime = Runtime::new(Widget::box_(size, Color::WHITE)).unwrap();
    runtime.run_frame_at(bounds, Instant::now()).unwrap();
    let second_id = Rc::new(Cell::new(None));
    let callbacks = Rc::new(RefCell::new(Vec::new()));

    schedule_frame_callback({
        let second_id = second_id.clone();
        let callbacks = callbacks.clone();
        move |_| {
            callbacks.borrow_mut().push('A');
            let _ = cancel_frame_callback(second_id.get().unwrap());
        }
    });
    second_id.set(Some(schedule_frame_callback({
        let callbacks = callbacks.clone();
        move |_| callbacks.borrow_mut().push('B')
    })));
    schedule_frame_callback({
        let callbacks = callbacks.clone();
        move |_| callbacks.borrow_mut().push('C')
    });

    runtime.run_frame_at(bounds, Instant::now()).unwrap();

    assert_eq!(callbacks.borrow().as_slice(), &['A', 'C']);
}
