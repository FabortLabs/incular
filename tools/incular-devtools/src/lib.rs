//! Standalone Incular DevTools application and its testable model surfaces.

use std::sync::{Arc, Mutex};

pub mod activity;
pub mod inspector;
pub mod performance;
mod report;
pub mod session;
mod transport;
pub mod views;

/// Runs the standalone DevTools application against the selected target.
pub fn run() {
    match create_application(session::requested_target_pid(std::env::args_os())) {
        Ok(application) => {
            if let Err(error) = incular::run(application) {
                eprintln!("DevTools runtime: {error:?}");
            }
        }
        Err(error) => eprintln!("unable to start DevTools: {error:?}"),
    }
}

/// Constructs the desktop inspector, including a connection workspace when no target is running.
pub fn create_application(
    target_pid: Option<u32>,
) -> Result<incular::runtime::Application, incular::runtime::WindowError> {
    create_application_with_options(
        target_pid,
        incular::prelude::WindowOptions {
            title: "Incular DevTools".into(),
            initial_logical_size: incular::prelude::Size::new(1440., 900.),
            minimum_logical_size: Some(incular::prelude::Size::new(960., 640.)),
            ..Default::default()
        },
    )
}

/// Constructs the inspector with caller-supplied native window options.
pub fn create_application_with_options(
    target_pid: Option<u32>,
    options: incular::prelude::WindowOptions,
) -> Result<incular::runtime::Application, incular::runtime::WindowError> {
    let report = session::scan_sessions();
    for warning in &report.warnings {
        eprintln!("DevTools discovery: {warning}");
    }
    let record = session::select_session(&report.sessions, target_pid);
    let shared = Arc::new(Mutex::new(inspector::InspectorModel::default()));
    let bridge = transport::start_client(record, target_pid, Arc::clone(&shared));
    views::create_application(shared, bridge, options)
}
