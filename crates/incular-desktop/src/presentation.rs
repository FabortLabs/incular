//! Per-window GPU submission through the shared host core, plus the desktop
//! diagnostics that need Winit (monitor refresh rate) or the DevTools agent.
use crate::window_host::NativeWindowState;
use incular_host::PresentOutcome;
use incular_rendering::DisplayList;
use incular_runtime::Application;
use incular_wgpu::RendererError;

pub(crate) fn present_window(
    application: &mut Application,
    state: &mut NativeWindowState,
    list: &DisplayList,
    #[cfg(feature = "devtools")] frame: &incular_runtime::FrameStats,
    #[cfg(feature = "devtools")] devtools: &mut crate::devtools_runner::DevToolsState,
) {
    match state.surface.present(application, list) {
        PresentOutcome::Presented(stats) => {
            #[cfg(feature = "devtools")]
            record_devtools_frame(application, state, &stats, frame, devtools);
            #[cfg(not(feature = "devtools"))]
            let _ = stats;
        }
        PresentOutcome::Skipped => {}
        PresentOutcome::Failed(error) => {
            let message = match error {
                RendererError::OutOfMemory => "renderer stopped: out of GPU memory".to_owned(),
                error => format!("renderer error: {error}"),
            };
            eprintln!("Incular {message}");
            #[cfg(feature = "devtools")]
            let _ = devtools.push_frame(incular_devtools_protocol::TargetEvent::Log {
                level: "error".into(),
                target: "incular::renderer".into(),
                message,
            });
        }
    }
}

#[cfg(feature = "devtools")]
fn record_devtools_frame(
    application: &mut Application,
    state: &NativeWindowState,
    stats: &incular_wgpu::RenderStats,
    frame: &incular_runtime::FrameStats,
    devtools: &mut crate::devtools_runner::DevToolsState,
) {
    let id = state.surface.id();
    let renderer = state.surface.renderer();
    let budget_us = state
        .window
        .current_monitor()
        .and_then(|monitor| monitor.refresh_rate_millihertz())
        .filter(|rate| *rate > 0)
        .map(|rate| u32::try_from(1_000_000_000_u64 / u64::from(rate)).unwrap_or(u32::MAX));
    let cpu_total = frame
        .timings
        .cpu_total
        .saturating_add(stats.prepare_us)
        .saturating_add(stats.encode_us)
        .saturating_add(stats.submit_us);
    let gpu_us = renderer
        .gpu_frame_timings()
        .map(|timing| timing.main_pass_us);
    devtools.record_frame_sample(
        id,
        crate::devtools_overlay::FrameSample {
            cpu_us: cpu_total,
            gpu_us: gpu_us.map(|us| us.max(0.).round() as u32),
            budget_us,
        },
    );
    let frame_id = devtools.next_frame();
    let _ = devtools.push_frame(incular_devtools_protocol::TargetEvent::FrameRecord(
        incular_devtools_protocol::FrameRecordEvent {
            window: incular_devtools_protocol::DevWindowId::new(
                u64::from(id.index()) + 1,
                u64::from(id.generation()),
            ),
            frame: frame_id,
            timings: incular_devtools_protocol::FrameTimingsWire {
                event_processing: frame.timings.event_processing,
                runtime_messages: frame.timings.runtime_messages,
                build: frame.timings.build,
                layout: frame.timings.layout,
                composite: frame.timings.composite,
                semantics: frame.timings.semantics,
                paint: frame.timings.paint,
                cpu_total,
                prepare: stats.prepare_us,
                encode: stats.encode_us,
                submit: stats.submit_us,
                gpu_us,
            },
            budget_us,
            over_budget: budget_us.is_some_and(|budget| cpu_total > budget),
            draw_calls: stats.draw_calls,
            instances: stats.total_instances(),
            upload_bytes: stats.upload_bytes,
            pipelines_created: stats.pipelines_created,
        },
    ));
    if devtools.note_recorded_frame()
        && devtools.deep_recording()
        && let Some(trace) = application.devtools_take_deep_trace(id, frame_id)
    {
        let _ = devtools.push_frame(incular_devtools_protocol::TargetEvent::DeepTrace(trace));
    }
}
