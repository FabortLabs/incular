//! Text shaping and grayscale glyph-raster quality on a HiDPI window.
use incular::prelude::*;

// Local paper palette.
const DEMO_CANVAS: Color = Color::rgba(247, 243, 235, 255);
const DEMO_SURFACE: Color = Color::rgba(255, 253, 248, 255);
const DEMO_TEXT: Color = Color::rgba(46, 38, 32, 255);

#[path = "../tests/support/mod.rs"]
pub(crate) mod example_support;
#[path = "tests/simulations.rs"]
pub(crate) mod simulations;

fn main() {
    let app = Application::new(|_| {
        let content: Widget = {
            Widget::from(Column::new(Vec::<Widget>::from([
            Text::new("Incular Typography")
                .style(TextStyle { inherit: false,
                    size: 28.0,
                    color: Color::rgba(158, 62, 34, 255),
                    ..TextStyle::default()
                })
                .into(),
            Text::new("12 px — H E F   O C S G   A V W M   0123456789")
                .style(TextStyle { inherit: false,
                    size: 12.0,
                    color: DEMO_TEXT,
                    ..TextStyle::default()
                })
                .into(),
            Text::new("14 px — The quick brown fox jumps over the lazy dog.")
                .style(TextStyle { inherit: false,
                    size: 14.0,
                    color: DEMO_TEXT,
                    ..TextStyle::default()
                })
                .into(),
            Text::new("16 px — H E F   O C S G   A V W M")
                .style(TextStyle { inherit: false,
                    size: 16.0,
                    color: DEMO_TEXT,
                    ..TextStyle::default()
                })
                .into(),
            Text::new("20 px — Smooth grayscale coverage")
                .style(TextStyle { inherit: false,
                    size: 20.0,
                    color: Color::rgba(12, 96, 123, 255),
                    ..TextStyle::default()
                })
                .into(),
            Text::new("28 px — Curves O C S G and diagonals A V W M")
                .style(TextStyle { inherit: false,
                    size: 28.0,
                    color: Color::rgba(114, 61, 114, 255),
                    ..TextStyle::default()
                })
                .into(),
            Text::new("40 px — Incular text")
                .style(TextStyle { inherit: false,
                    size: 40.0,
                    color: Color::rgba(158, 62, 34, 255),
                    ..TextStyle::default()
                })
                .into(),
            Text::new("56 px — AVOCS")
                .style(TextStyle { inherit: false,
                    size: 56.0,
                    color: DEMO_TEXT,
                    ..TextStyle::default()
                })
                .into(),
            Text::new(
                "Small text wraps in logical pixels; the GPU raster cache uses physical pixels.",
            )
            .style(TextStyle { inherit: false,
                size: 14.0,
                color: Color::rgba(112, 96, 80, 255),
                ..TextStyle::default()
            })
            .into(),
            Text::new("Unicode: café — नमस्ते — 世界 — emoji 🙂")
                .style(TextStyle { inherit: false,
                    size: 20.0,
                    color: Color::rgba(12, 96, 123, 255),
                    ..TextStyle::default()
                })
                .into(),
            Text::new("Font fallback: Hello — नमस्ते — 日本語 — مرحبا")
                .style(TextStyle { inherit: false,
                    size: 20.0,
                    color: Color::rgba(32, 111, 89, 255),
                    ..TextStyle::default()
                })
                .into(),
            Text::new("Greek Ελληνικά · Cyrillic Привет · Hebrew שלום · Thai สวัสดี")
                .style(TextStyle { inherit: false,
                    size: 16.0,
                    color: Color::rgba(114, 61, 114, 255),
                    ..TextStyle::default()
                })
                .into(),
            DecoratedBox::new(
                Text::new("Dark text on a light rectangle").style(TextStyle { inherit: false,
                    size: 20.0,
                    color: Color::rgba(25, 30, 40, 255),
                    ..TextStyle::default()
                }),
            )
            .background(DEMO_SURFACE)
            .into(),
        ])))
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
    .expect("valid typography application");
    example_support::spawn_if_requested(app.simulation(), simulations::run);
    incular::run(app).expect("native typography application");
}
