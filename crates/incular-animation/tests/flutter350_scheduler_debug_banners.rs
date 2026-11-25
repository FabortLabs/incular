use std::{cell::RefCell, panic::catch_unwind, rc::Rc};

use incular_animation::scheduler::{
    SchedulerDebugFlags, add_post_frame_callback, begin_frame, clear_scheduler_debug_output,
    debug_assert_all_scheduler_vars_unset, end_frame, scheduler_debug_flags,
    set_scheduler_debug_flags, set_scheduler_debug_output, try_assert_all_scheduler_vars_unset,
};
use incular_core::time::Instant;

#[test]
fn debug_assertion_tracks_begin_and_end_flags() {
    let reason = "Example test";
    for (begin, end, clean) in [
        (false, false, true),
        (true, false, false),
        (false, true, false),
        (false, false, true),
    ] {
        let flags = SchedulerDebugFlags {
            print_begin_frame_banner: begin,
            print_end_frame_banner: end,
        };
        set_scheduler_debug_flags(flags);
        assert_eq!(scheduler_debug_flags(), flags);
        let checked = try_assert_all_scheduler_vars_unset(reason);
        assert_eq!(checked.is_ok(), clean);
        if let Err(error) = checked {
            assert_eq!(error.to_string(), reason);
        }
        let debug = catch_unwind(|| debug_assert_all_scheduler_vars_unset(reason));
        assert_eq!(debug.is_ok(), clean || !cfg!(debug_assertions));
    }
}

#[test]
fn banners_bracket_the_callbacks_and_are_silent_by_default() {
    let captured = Rc::new(RefCell::new(Vec::new()));
    set_scheduler_debug_output({
        let captured = captured.clone();
        move |message| captured.borrow_mut().push(message.to_owned())
    });
    let now = Instant::now();
    begin_frame(now);
    end_frame(now);
    assert!(captured.borrow().is_empty());

    set_scheduler_debug_flags(SchedulerDebugFlags {
        print_begin_frame_banner: true,
        print_end_frame_banner: true,
    });
    add_post_frame_callback({
        let captured = captured.clone();
        move |_| captured.borrow_mut().push("post-frame callback".to_owned())
    });
    begin_frame(now);
    end_frame(now);
    let entries = captured.take();
    if cfg!(debug_assertions) {
        assert_eq!(entries.len(), 3);
        assert!(entries[0].starts_with("BEGIN FRAME #"));
        assert_eq!(entries[1], "post-frame callback");
        assert!(entries[2].starts_with("END FRAME #"));
    } else {
        assert_eq!(entries, ["post-frame callback"]);
    }
    set_scheduler_debug_flags(SchedulerDebugFlags::default());
    clear_scheduler_debug_output();
}
