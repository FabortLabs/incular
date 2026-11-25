//! Ambient configuration and localization behavior tests.

use incular_config::{Locale, LocalizationCatalog, RuntimeEnvironment, TextDirection};
use incular_widgets::{Localizations, MediaQuery, MediaQueryData, Widget};

struct Catalog;

impl LocalizationCatalog for Catalog {
    fn supported_locales(&self) -> &[Locale] {
        static LOCALES: std::sync::OnceLock<Vec<Locale>> = std::sync::OnceLock::new();
        LOCALES
            .get_or_init(|| vec!["en".parse().expect("valid locale")])
            .as_slice()
    }

    fn message(&self, locale: &Locale, key: &str) -> Option<&str> {
        (locale.to_string() == "en" && key == "greeting").then_some("Hello")
    }
}

#[test]
fn localizations_use_icu_fallback_and_direction() {
    let localizations = Localizations::new(
        "en-GB".parse().expect("valid locale"),
        Catalog,
        incular_widgets::Column::new(Vec::<Widget>::new()),
    );
    assert_eq!(localizations.text("greeting").as_deref(), Some("Hello"));
    assert_eq!(localizations.text_direction(), TextDirection::Ltr);
    assert_eq!(
        localizations
            .resolve("greeting")
            .unwrap()
            .locale
            .to_string(),
        "en"
    );
}

#[test]
fn media_query_data_is_the_normalized_runtime_environment() {
    let data = MediaQueryData {
        viewport: incular_core::Size::new(640., 480.),
        scale_factor: f64::NAN,
        brightness: incular_config::Brightness::Dark,
        ..RuntimeEnvironment::default()
    };
    let query = MediaQuery::new(data, incular_widgets::Column::new(Vec::<Widget>::new()));
    assert_eq!(query.data().viewport, incular_core::Size::new(640., 480.));
    assert_eq!(query.data().scale_factor, 1.);
    assert_eq!(query.data().brightness, incular_config::Brightness::Dark);
}

#[test]
fn media_query_resize_updates_consumers() {
    let child: Widget = incular_widgets::Column::new(Vec::<Widget>::new()).into();
    let first = MediaQuery::new(
        RuntimeEnvironment {
            viewport: incular_core::Size::new(320., 240.),
            ..RuntimeEnvironment::default()
        },
        child.clone(),
    );
    let second = MediaQuery::new(
        RuntimeEnvironment {
            viewport: incular_core::Size::new(800., 600.),
            ..RuntimeEnvironment::default()
        },
        child,
    );
    assert_ne!(first.data().viewport, second.data().viewport);
}

#[test]
fn media_query_brightness_update() {
    let child: Widget = incular_widgets::Column::new(Vec::<Widget>::new()).into();
    let light = MediaQuery::new(RuntimeEnvironment::default(), child.clone());
    let dark = MediaQuery::new(
        RuntimeEnvironment {
            brightness: incular_config::Brightness::Dark,
            ..RuntimeEnvironment::default()
        },
        child,
    );
    assert_ne!(light.data().brightness, dark.data().brightness);
}

#[test]
fn locale_change_rebuilds_only_consumers() {
    let localizations = Localizations::new(
        "en".parse().expect("valid locale"),
        Catalog,
        incular_widgets::Column::new(Vec::<Widget>::new()),
    );
    let changed = localizations
        .clone()
        .with_locale("ar".parse().expect("valid locale"));
    assert_eq!(localizations.locale().to_string(), "en");
    assert_eq!(changed.locale().to_string(), "ar");
    assert_eq!(changed.text_direction(), TextDirection::Rtl);
}

/// Flutter's `MediaQuery.sizeOf` and friends (`widgets/media_query.dart`
/// 1141-1143, 2094-2104): a reader depends on one aspect, so a change to an
/// unrelated aspect does not rebuild it. The whole-value `MediaQueryData`
/// reader still rebuilds on any change.
#[test]
fn media_query_aspect_readers_rebuild_only_for_their_aspect() {
    use std::{cell::Cell, rc::Rc};

    use incular_config::Constraints;
    use incular_core::{Color, Size};
    use incular_widgets::{BuildContext, Builder, internal::WidgetTree};

    fn counted(
        builds: &Rc<Cell<u32>>,
        read: impl Fn(&BuildContext<'_>) -> f32 + 'static,
    ) -> Widget {
        let builds = builds.clone();
        Builder::new(move |context| {
            builds.set(builds.get() + 1);
            Widget::box_(Size::new(read(context), 10.), Color::WHITE)
        })
        .into()
    }

    let counts: Vec<Rc<Cell<u32>>> = (0..4).map(|_| Rc::new(Cell::new(0))).collect();
    // The same child each time, so only environment notifications rebuild.
    let child: Widget = incular_widgets::Column::new([
        counted(&counts[0], |c| c.media_query_size().map_or(0., |s| s.width)),
        counted(&counts[1], |c| {
            c.media_query_device_pixel_ratio().map_or(0., |v| v as f32)
        }),
        counted(&counts[2], |c| c.media_query_text_scale().unwrap_or(0.)),
        counted(&counts[3], |c| {
            c.depend_on::<MediaQueryData>()
                .map_or(0., |data| data.viewport.width)
        }),
    ])
    .into();
    let environment = |viewport: f32, scale_factor: f64, text_scale: f32| RuntimeEnvironment {
        viewport: Size::new(viewport, 200.),
        scale_factor,
        text_scale,
        ..RuntimeEnvironment::default()
    };
    let scoped =
        |data: RuntimeEnvironment| -> Widget { MediaQuery::new(data, child.clone()).into() };
    let counts_now = || counts.iter().map(|c| c.get()).collect::<Vec<_>>();
    let layout = |tree: &mut WidgetTree| {
        tree.layout(Constraints::loose(Size::new(500., 500.)))
            .expect("layout");
    };

    let mut tree = WidgetTree::new();
    let root = tree
        .mount(scoped(environment(320., 1., 1.)))
        .expect("mount");
    layout(&mut tree);
    assert_eq!(counts_now(), [1, 1, 1, 1]);

    // Device pixel ratio only: the size, text scale readers keep their builds.
    tree.update(root, scoped(environment(320., 2., 1.)))
        .expect("update");
    layout(&mut tree);
    assert_eq!(counts_now(), [1, 2, 1, 2]);

    // Text scale only.
    tree.update(root, scoped(environment(320., 2., 1.5)))
        .expect("update");
    layout(&mut tree);
    assert_eq!(counts_now(), [1, 2, 2, 3]);

    // Size: the size reader rebuilds and reads the new value.
    tree.update(root, scoped(environment(640., 2., 1.5)))
        .expect("update");
    layout(&mut tree);
    assert_eq!(counts_now(), [2, 2, 2, 4]);
}

#[test]
fn widgets_app_title_uses_basic_locale_precedence() {
    use incular_core::Color;
    use incular_widgets::{TitleController, WidgetsApp};

    let title = TitleController::new("fallback", Color::BLACK);
    let locales = |tags: &[&str]| {
        tags.iter()
            .map(|tag| tag.parse::<Locale>().unwrap())
            .collect::<Vec<_>>()
    };
    let app = WidgetsApp::empty()
        .supported_locales(locales(&["en-US", "fr-TN", "zh-CN", "zh-Hant-TW"]))
        .title_controller(title.clone())
        .on_generate_title(Locale::to_string);
    for (preferred, expected) in [
        (&["zh", "zh-Hant-TW"][..], "zh-Hant-TW"),
        (&["zh", "en-US"][..], "zh-CN"),
        (&["de", "ar-TN"][..], "fr-TN"),
        (&[][..], "en-US"),
    ] {
        let _ = app.build_with_environment(RuntimeEnvironment {
            locales: locales(preferred),
            ..Default::default()
        });
        assert_eq!(title.title(), expected);
    }
}
