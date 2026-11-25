//! Optional scheduler phase banners and their test assertion.

use std::{
    cell::{Cell, RefCell},
    fmt,
    rc::Rc,
};

use incular_core::time::Instant;

type DebugOutput = Rc<dyn Fn(&str)>;

thread_local! {
    static FLAGS: Cell<SchedulerDebugFlags> = const { Cell::new(SchedulerDebugFlags {
        print_begin_frame_banner: false,
        print_end_frame_banner: false,
    }) };
    static OUTPUT: RefCell<Option<DebugOutput>> = const { RefCell::new(None) };
    static FRAME_NUMBER: Cell<u64> = const { Cell::new(0) };
}

/// The frame banners guarded by Flutter's scheduler debug assertion.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SchedulerDebugFlags {
    pub print_begin_frame_banner: bool,
    pub print_end_frame_banner: bool,
}

impl SchedulerDebugFlags {
    fn any_enabled(self) -> bool {
        self.print_begin_frame_banner || self.print_end_frame_banner
    }
}

/// Sets the scheduler's debug-only frame banners for this UI thread.
pub fn set_scheduler_debug_flags(flags: SchedulerDebugFlags) {
    FLAGS.with(|current| current.set(flags));
}

/// The current thread's scheduler frame-banner switches.
#[must_use]
pub fn scheduler_debug_flags() -> SchedulerDebugFlags {
    FLAGS.with(Cell::get)
}

/// Captures frame banners instead of writing them to stderr.
pub fn set_scheduler_debug_output(output: impl Fn(&str) + 'static) {
    drop(OUTPUT.with(|current| current.replace(Some(Rc::new(output)))));
}

/// Restores the default stderr output.
pub fn clear_scheduler_debug_output() {
    drop(OUTPUT.with(|current| current.take()));
}

/// A frame debug switch was left enabled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SchedulerDebugFlagsEnabled<'a> {
    pub reason: &'a str,
    pub flags: SchedulerDebugFlags,
}

impl fmt::Display for SchedulerDebugFlagsEnabled<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.reason)
    }
}

impl std::error::Error for SchedulerDebugFlagsEnabled<'_> {}

/// Checks the scheduler debug flags even in a release build.
///
/// # Errors
/// Returns the supplied reason and flags if either banner is enabled.
pub fn try_assert_all_scheduler_vars_unset(
    reason: &str,
) -> Result<(), SchedulerDebugFlagsEnabled<'_>> {
    let flags = scheduler_debug_flags();
    if flags.any_enabled() {
        Err(SchedulerDebugFlagsEnabled { reason, flags })
    } else {
        Ok(())
    }
}

/// Flutter's `debugAssertAllSchedulerVarsUnset` check.
///
/// # Panics
/// Panics with the supplied reason if a scheduler frame banner is enabled
/// in a debug build. Use [`try_assert_all_scheduler_vars_unset`] to inspect
/// the current flags without panicking.
#[must_use]
pub fn debug_assert_all_scheduler_vars_unset(reason: &str) -> bool {
    debug_assert!(
        try_assert_all_scheduler_vars_unset(reason).is_ok(),
        "{reason}"
    );
    true
}

fn emit(message: &str) {
    let output = OUTPUT.with(|current| current.borrow().clone());
    if let Some(output) = output {
        output(message);
    } else {
        eprintln!("{message}");
    }
}

pub(crate) fn begin_banner(timestamp: Instant) {
    if !cfg!(debug_assertions) {
        return;
    }
    let flags = scheduler_debug_flags();
    let number = FRAME_NUMBER.with(|number| {
        let next = number.get().wrapping_add(1);
        number.set(next);
        next
    });
    if flags.print_begin_frame_banner {
        emit(&format!("BEGIN FRAME #{number} ({timestamp:?})"));
    }
}

pub(crate) fn end_banner() {
    if cfg!(debug_assertions) && scheduler_debug_flags().print_end_frame_banner {
        let number = FRAME_NUMBER.with(Cell::get);
        emit(&format!("END FRAME #{number}"));
    }
}
