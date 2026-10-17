//! A single app that exercises every part of Incular DevTools: overlays, FPS,
//! animation speed, property editing, semantics, layers, caches and signals.
//!
//! ```text
//! cargo run -p incular --example devtools_playground --features devtools
//! ```
//!
//! Start it with `INCULAR_DEVTOOLS=1` (agent only) or `--devtools` (which also
//! opens a DevTools window). Without the `devtools` feature it still runs as an
//! ordinary demo, but the named signals are not registered.
use incular::controls_prelude::{
    ButtonStyle, ButtonVariant, ControlTheme, ControlThemeScope, GhostButton, PrimaryButton,
};
use incular::prelude::*;

pub mod console_lab;
mod editable;
mod jank;
mod layout_lab;
mod motion;
mod semantics_lab;
mod signals_lab;
mod text_lab;
mod theme;

use theme::*;

/// One page of the playground, selected from the sidebar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    Motion,
    Jank,
    Editable,
    Signals,
    Layout,
    Text,
    Semantics,
    Console,
}

impl Section {
    pub const ALL: [Self; 8] = [
        Self::Motion,
        Self::Jank,
        Self::Editable,
        Self::Signals,
        Self::Layout,
        Self::Text,
        Self::Semantics,
        Self::Console,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Self::Motion => "Live motion",
            Self::Jank => "Jank lab",
            Self::Editable => "Editable widgets",
            Self::Signals => "Signals",
            Self::Layout => "Layout & scroll",
            Self::Text => "Text",
            Self::Semantics => "Semantics",
            Self::Console => "Console",
        }
    }
}

/// Everything the sections keep alive between rebuilds.
#[derive(Clone)]
struct Playground {
    section: Signal<Section>,
    motion: motion::Motion,
    jank: jank::Jank,
    signals: signals_lab::Signals,
    text: text_lab::TextLab,
    semantics: semantics_lab::SemanticsLab,
    console: console_lab::ConsoleLab,
}

impl Playground {
    fn new(section: Section) -> Self {
        Self {
            section: Signal::new(section),
            motion: motion::Motion::new(),
            jank: jank::Jank::new(),
            signals: signals_lab::Signals::new(),
            text: text_lab::TextLab::new(),
            semantics: semantics_lab::SemanticsLab::new(),
            console: console_lab::ConsoleLab::new(),
        }
    }

    fn page(&self, section: Section) -> Widget {
        match section {
            Section::Motion => motion::view(&self.motion),
            Section::Jank => jank::view(&self.jank),
            Section::Editable => editable::view(),
            Section::Signals => signals_lab::view(&self.signals),
            Section::Layout => layout_lab::view(),
            Section::Text => text_lab::view(&self.text),
            Section::Semantics => semantics_lab::view(&self.semantics),
            Section::Console => console_lab::view(&self.console),
        }
    }
}

/// The two overlapping circles of the Incular mark.
fn logo() -> Widget {
    let circle = |size: f32| {
        Container::new()
            .width(size)
            .height(size)
            .radius(size / 2.)
            .color(VIOLET)
    };
    SizedBox::new()
        .width(34.)
        .height(34.)
        .child(Stack::new([
            Positioned::new(circle(20.)).left(0.).top(0.),
            Positioned::new(
                Container::new()
                    .width(22.)
                    .height(22.)
                    .radius(11.)
                    .color(Color::rgba(128, 0, 255, 200))
                    .border(RenderBorder::new(2., SURFACE)),
            )
            .left(12.)
            .top(12.),
        ]))
        .into()
}

fn header() -> Widget {
    Container::new()
        .height(64.)
        .padding(EdgeInsets::symmetric(24., 0.))
        .color(SURFACE)
        .border(RenderBorder::new(1., LINE))
        .alignment(Alignment::CENTER_LEFT)
        .child(Row::new([
            logo(),
            hgap(12.),
            text("incular", 24., FontWeight::W800, VIOLET),
            hgap(10.),
            text("DevTools playground", 15., FontWeight::W500, MUTED),
            Widget::from(Spacer::new()),
            container_hint(),
        ]))
        .into()
}

fn container_hint() -> Widget {
    Container::new()
        .padding(EdgeInsets::symmetric(12., 7.))
        .radius(8.)
        .color(VIOLET_SOFT)
        .child(text(
            "Run with --devtools or INCULAR_DEVTOOLS=1",
            12.5,
            FontWeight::W600,
            VIOLET,
        ))
        .into()
}

fn nav_style(variant: ButtonVariant) -> ButtonStyle {
    ButtonStyle::new()
        .variant(variant)
        .fixed_size(Size::new(186., 38.))
}

fn sidebar(playground: &Playground) -> Widget {
    let current = playground.section.get();
    let mut items = vec![text("SECTIONS", 11., FontWeight::W700, MUTED), gap(8.)];
    for section in Section::ALL {
        let signal = playground.section.clone();
        let item: Widget = if section == current {
            PrimaryButton::new(section.title())
                .style(nav_style(ButtonVariant::Primary))
                .on_click(move || {
                    signal.set(section);
                })
                .into()
        } else {
            GhostButton::new(section.title())
                .style(nav_style(ButtonVariant::Ghost))
                .on_click(move || {
                    signal.set(section);
                })
                .into()
        };
        items.push(item);
        items.push(gap(4.));
    }
    Container::new()
        .width(220.)
        .padding(EdgeInsets::all(16.))
        .color(SURFACE)
        .border(RenderBorder::new(1., LINE))
        .child(Column::new(items).cross_axis_alignment(CrossAxisAlignment::Stretch))
        .into()
}

fn root(playground: &Playground) -> Widget {
    let theme = ControlTheme::light();
    let current = playground.section.get();
    let content = Column::new([
        header(),
        Widget::from(Expanded::new(
            Row::new([
                sidebar(playground),
                Widget::from(Expanded::new(playground.page(current))),
            ])
            .cross_axis_alignment(CrossAxisAlignment::Stretch),
        )),
    ])
    .cross_axis_alignment(CrossAxisAlignment::Stretch);
    let themed = ControlThemeScope::new(
        theme.clone(),
        Container::new().color(BACKGROUND).child(content),
    );
    DefaultTextStyle::new(
        TextStyle::new().color(INK).font_size(14.),
        Widget::from(themed),
    )
    .into()
}

/// Builds the playground opened on `section`.
pub fn build_app_at(section: Section) -> Application {
    let mut options = WindowOptions::new("Incular DevTools Playground");
    options.initial_logical_size = Size::new(1240., 820.);
    options.minimum_logical_size = Some(Size::new(900., 600.));
    let playground = Playground::new(section);
    Application::new_with_options(options, move |_cx| -> Widget {
        Directionality::new(TextDirection::Ltr, root(&playground)).into()
    })
    .expect("valid devtools playground application")
}

/// `INCULAR_PLAYGROUND_SECTION=jank` (any title word, case-insensitive) opens that page.
fn initial_section() -> Section {
    let wanted = std::env::var("INCULAR_PLAYGROUND_SECTION")
        .unwrap_or_default()
        .to_lowercase();
    Section::ALL
        .into_iter()
        .find(|section| !wanted.is_empty() && section.title().to_lowercase().starts_with(&wanted))
        .unwrap_or(Section::Motion)
}

/// `INCULAR_PLAYGROUND_LOG_DEMO=1` logs a message from a background thread
/// every two seconds, to watch the Console without clicking.
fn spawn_log_demo() {
    if std::env::var_os("INCULAR_PLAYGROUND_LOG_DEMO").is_none() {
        return;
    }
    std::thread::spawn(|| {
        let mut tick = 0_u32;
        loop {
            tick = tick.wrapping_add(1);
            incular::devtools::log(
                incular::devtools::LogLevel::Info,
                "playground::demo",
                format!("background heartbeat {tick}"),
            );
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
    });
}

fn main() {
    incular::devtools::log(
        incular::devtools::LogLevel::Info,
        "playground",
        "DevTools playground started",
    );
    spawn_log_demo();
    incular::run(build_app_at(initial_section())).expect("native devtools playground");
}
