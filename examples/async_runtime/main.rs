//! Tokio runtime demo: component-scoped work, cancellation, a blocking job,
//! and UI-thread-only signal updates.
use std::{cell::Cell, rc::Rc, time::Duration};

use incular::prelude::*;

// Local lagoon palette.
const DEMO_CANVAS: Color = Color::rgba(14, 32, 33, 255);
const DEMO_TEXT: Color = Color::rgba(235, 247, 236, 255);
const DEMO_ACCENT: Color = Color::rgba(125, 224, 188, 255);

#[path = "../tests/support/mod.rs"]
pub(crate) mod example_support;
#[path = "tests/simulations.rs"]
pub(crate) mod simulations;

fn main() {
    // The root builder is a read-only reactive scope. Start in Loading so the
    // first build can render directly without mutating a signal.
    let state = Signal::new(AsyncValue::<String>::Loading);
    let started = Rc::new(Cell::new(false));
    let app = Application::new(move |cx| {
        let content: Widget = {
            if !started.replace(true) {
                let after_timer = state.clone();
                cx.spawn_into(
                    async {
                        tokio::time::sleep(Duration::from_millis(650)).await;
                        "Tokio timer completed; UI update ran on the native owner".to_owned()
                    },
                    move |result, _| {
                        match result {
                            Ok(message) => after_timer.set(AsyncValue::Ready(message)),
                            Err(error) => after_timer.set(AsyncValue::Error(error)),
                        };
                    },
                );

                // A scoped task is cancelled before it can update a stale owner.
                let scope = cx.task_scope();
                let cancelled = cx.spawn_in(&scope, async move {
                    tokio::time::sleep(Duration::from_secs(30)).await;
                });
                cancelled.cancel();
                scope.cancel();

                let after_blocking = state.clone();
                cx.spawn_blocking(
                    || (1_u64..=25_000).sum::<u64>(),
                    move |result, _runtime| {
                        match result {
                            Ok(sum) => after_blocking
                                .set(AsyncValue::Ready(format!("background sum finished: {sum}"))),
                            Err(error) => after_blocking.set(AsyncValue::Error(error)),
                        };
                    },
                );
            }

            let message = match state.get() {
                AsyncValue::Idle => "idle".into(),
                AsyncValue::Loading => "loading local work…".into(),
                AsyncValue::Ready(value) => value,
                AsyncValue::Error(error) => format!("task error: {error:?}"),
            };
            Padding::all(
                28.,
                Widget::from(Column::new(Vec::<Widget>::from([
                    Text::new("Incular async runtime")
                        .style(TextStyle {
                            inherit: false,
                            size: 28.,
                            color: DEMO_ACCENT,
                            ..TextStyle::default()
                        })
                        .into(),
                    Text::new(
                        "The window remains input responsive; work wakes Winit only when ready.",
                    )
                    .into(),
                    Padding::all(16., Text::new(message)).into(),
                ]))),
            )
            .into()
        };
        Container::new()
            .background(DEMO_CANVAS)
            .alignment(Alignment::TOP_LEFT)
            .child(DefaultTextStyle::new(
                TextStyle::new().color(DEMO_TEXT),
                content,
            ))
            .into()
    })
    .expect("valid async application");
    // Advanced libraries can use Tokio directly. Direct tasks have Tokio/app
    // lifetime, not a declarative component lifetime.
    app.tokio_handle().spawn(async {});
    example_support::spawn_if_requested(app.simulation(), simulations::run);
    incular::run(app).expect("native async runtime application");
}
