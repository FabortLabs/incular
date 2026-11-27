use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use incular_config::{Brightness, Constraints};
use incular_core::{Color, Size};
use incular_rendering::{Brush, PaintCommand};
use incular_widgets::internal::WidgetTree;
use incular_widgets::widget_previews::{MultiPreviewThemeData, PreviewBuilder, PreviewThemeData};
use incular_widgets::{BuildContext, Builder, ColoredBox, SizedBox, Widget};

const INNER: Color = Color::rgba(255, 0, 0, 255);
const OUTER: Color = Color::rgba(0, 0, 255, 255);
const LEAF: Color = Color::rgba(0, 255, 0, 255);

fn leaf() -> Widget {
    ColoredBox::new(LEAF, SizedBox::new().width(40.).height(20.)).into()
}

fn painted_colors(widget: impl Into<Widget>) -> Vec<Color> {
    let mut tree = WidgetTree::new();
    tree.mount(widget.into()).expect("mount preview");
    tree.layout(Constraints::loose(Size::new(100., 100.)))
        .expect("layout preview");
    tree.paint()
        .commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::Rect { color, .. } => Some(*color),
            PaintCommand::RRect {
                brush: Brush::Solid(color),
                ..
            }
            | PaintCommand::FillPath {
                brush: Brush::Solid(color),
                ..
            } => Some(*color),
            _ => None,
        })
        .collect()
}

#[test]
fn last_added_preview_wrapper_paints_outside_the_first() {
    let mut builder = PreviewBuilder::new();
    let log = Rc::new(RefCell::new(Vec::new()));
    for (name, color) in [("old", INNER), ("new", OUTER)] {
        let log = log.clone();
        builder.add_wrapper(move |child| {
            log.borrow_mut().push(name);
            ColoredBox::new(color, child).into()
        });
    }
    let preview = builder.build();
    assert_eq!(preview.group(), "Default");
    assert!(log.borrow().is_empty());
    assert_eq!(painted_colors(preview.wrap(leaf())), [OUTER, INNER, LEAF]);
    assert_eq!(*log.borrow(), ["old", "new"]);
}

struct MarkTheme(Color);

impl PreviewThemeData for MarkTheme {
    fn apply(&self, _: &BuildContext<'_>, child: Widget) -> Widget {
        ColoredBox::new(self.0, child).into()
    }
}

#[test]
fn first_theme_is_outermost_and_factory_runs_only_on_application() {
    let first: Rc<dyn PreviewThemeData> = Rc::new(MarkTheme(OUTER));
    let second: Rc<dyn PreviewThemeData> = Rc::new(MarkTheme(INNER));
    let themes = MultiPreviewThemeData::new([first, second]);
    let calls = Rc::new(Cell::new(0));
    let mut builder = PreviewBuilder::new();
    builder.theme = Some(Rc::new({
        let calls = calls.clone();
        move || {
            calls.set(calls.get() + 1);
            Rc::new(themes.clone())
        }
    }));
    let preview = builder.build();
    assert_eq!(calls.get(), 0);
    assert_eq!(
        painted_colors(Builder::new(move |ctx| preview.apply_theme(ctx, leaf()))),
        [OUTER, INNER, LEAF]
    );
    assert_eq!(calls.get(), 1);
}

#[test]
fn round_trip_builder_isolates_metadata_and_wrappers() {
    let mut builder = PreviewBuilder::new();
    builder.name = Some("original".to_owned());
    builder.size = Some(Size {
        width: 120.,
        height: f32::INFINITY,
    });
    builder.text_scale_factor = Some(1.5);
    builder.brightness = Some(Brightness::Dark);
    builder.add_wrapper(|child| ColoredBox::new(INNER, child).into());
    let original = builder.build();

    let mut clone = original.to_builder();
    assert_eq!(clone.group.as_deref(), Some("Default"));
    clone.name = Some("changed".to_owned());
    clone.set_wrapper(None);
    let changed = clone.build();
    assert_eq!(painted_colors(original.wrap(leaf())), [INNER, LEAF]);
    assert_eq!(painted_colors(changed.wrap(leaf())), [LEAF]);
    assert_eq!(original.name(), Some("original"));
    assert_eq!(changed.name(), Some("changed"));
    assert_eq!(changed.size(), builder.size);
    assert_eq!(changed.text_scale_factor(), Some(1.5));
    assert_eq!(changed.brightness(), Some(Brightness::Dark));
    builder.group = Some(String::new());
    assert_eq!(builder.build().group(), "");
}

struct Greeting(Vec<incular_config::Locale>);

impl incular_config::LocalizationCatalog for Greeting {
    fn supported_locales(&self) -> &[incular_config::Locale] {
        &self.0
    }

    fn message(&self, locale: &incular_config::Locale, key: &str) -> Option<&str> {
        if key != "hello" {
            return None;
        }
        match locale.to_string().as_str() {
            "en" => Some("Hello"),
            "es" => Some("Hola"),
            "ar" => Some("مرحبا"),
            _ => None,
        }
    }
}

#[test]
fn preview_locale_scope_retains_its_child_as_the_platform_locale_changes() {
    use incular_config::{Locale, LocalizationCatalog, RuntimeEnvironment, TextDirection};
    use incular_widgets::internal::Key;
    use incular_widgets::widget_previews::PreviewLocalizationsData;

    let locales = ["en", "es", "ar"].map(|tag| tag.parse::<Locale>().unwrap());
    let catalog: Rc<dyn LocalizationCatalog> = Rc::new(Greeting(locales.to_vec()));
    let target = Key::from("preview content");
    let seen = Rc::new(RefCell::new(Vec::new()));
    let child = Builder::new({
        let target = target.clone();
        let seen = seen.clone();
        move |context| {
            let (locale, catalog) = context
                .depend_on::<(Locale, Rc<dyn LocalizationCatalog>)>()
                .unwrap();
            seen.borrow_mut().push((
                locale.to_string(),
                catalog.message(&locale, "hello").unwrap().to_owned(),
                context.text_direction().unwrap(),
            ));
            leaf().with_key(target.clone())
        }
    });
    let calls = Rc::new(Cell::new(0));
    let mut builder = PreviewBuilder::new();
    builder.localizations = Some(Rc::new({
        let catalog = catalog.clone();
        let locales = locales.clone();
        let calls = calls.clone();
        move || {
            calls.set(calls.get() + 1);
            PreviewLocalizationsData {
                supported_locales: locales.to_vec(),
                catalog: Some(catalog.clone()),
                ..Default::default()
            }
        }
    }));
    let preview = builder.build();
    assert_eq!(calls.get(), 0);
    let mut tree = WidgetTree::new();
    let view = Constraints::loose(Size::new(100., 100.));
    tree.set_environment(RuntimeEnvironment {
        locales: vec![locales[0].clone()],
        ..Default::default()
    });
    tree.mount(preview.apply_localizations(child)).unwrap();
    tree.layout(view).unwrap();
    let old = tree.element_with_key(&target).unwrap();
    for locale in &locales[1..] {
        tree.set_environment(RuntimeEnvironment {
            locales: vec![locale.clone()],
            ..Default::default()
        });
        tree.layout(view).unwrap();
        assert_eq!(tree.element_with_key(&target), Some(old));
    }
    assert_eq!(
        *seen.borrow(),
        [
            ("en".to_owned(), "Hello".to_owned(), TextDirection::Ltr),
            ("es".to_owned(), "Hola".to_owned(), TextDirection::Ltr),
            ("ar".to_owned(), "مرحبا".to_owned(), TextDirection::Rtl),
        ]
    );
    assert_eq!(calls.get(), 1);
}

#[test]
fn preview_locale_resolution_prefers_an_explicit_locale_and_ordered_callbacks() {
    use incular_config::Locale;
    use incular_widgets::widget_previews::PreviewLocalizationsData;
    let locale = |tag: &str| tag.parse::<Locale>().unwrap();
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut data = PreviewLocalizationsData {
        locale: Some(locale("es")),
        supported_locales: vec![locale("en"), locale("es"), locale("ar")],
        locale_list_resolution: Some(Rc::new({
            let calls = calls.clone();
            move |preferred, _| {
                calls.borrow_mut().push("list");
                assert_eq!(preferred, [locale("es")]);
                None
            }
        })),
        locale_resolution: Some(Rc::new({
            let calls = calls.clone();
            move |preferred, supported| {
                calls.borrow_mut().push("single");
                assert_eq!(preferred, Some(&locale("es")));
                Some(supported[2].clone())
            }
        })),
        ..Default::default()
    };
    assert_eq!(data.resolve(&[locale("en")]), Some(locale("ar")));
    assert_eq!(*calls.borrow(), ["list", "single"]);
    data.locale_list_resolution = Some(Rc::new(|preferred, _| Some(preferred[0].clone())));
    assert_eq!(data.resolve(&[locale("en")]), Some(locale("es")));
    assert_eq!(*calls.borrow(), ["list", "single"]);
    assert_eq!(
        PreviewLocalizationsData::default().resolve(&[locale("ar")]),
        Some(locale("en-US"))
    );
}

#[test]
fn composed_preview_simulates_size_brightness_and_scale_for_wrappers_and_theme() {
    use incular_config::{EdgeInsets, Locale, RuntimeEnvironment};
    use incular_widgets::{
        Localizations, internal::Key, widget_previews::PreviewLocalizationsData,
    };

    let key = Key::from("preview geometry");
    let seen = Rc::new(RefCell::new(Vec::new()));
    let theme_seen = seen.clone();
    let wrapper_seen = seen.clone();
    let snapshot = |context: &BuildContext<'_>| {
        (
            context.media_query_size().unwrap(),
            context.media_query_text_scale().unwrap(),
            context.media_query_platform_brightness().unwrap(),
            context.media_query_padding().unwrap().top,
            Localizations::locale_of(context).to_string(),
        )
    };
    let mut builder = PreviewBuilder::new();
    builder.size = Some(Size {
        width: f32::INFINITY,
        height: 60.,
    });
    builder.brightness = Some(Brightness::Dark);
    builder.text_scale_factor = Some(1.75);
    builder.localizations = Some(Rc::new(|| PreviewLocalizationsData {
        locale: Some("es".parse::<Locale>().unwrap()),
        supported_locales: vec!["es".parse().unwrap()],
        ..Default::default()
    }));
    builder.theme = Some(Rc::new(move || {
        let seen = theme_seen.clone();
        Rc::new(move |context: &BuildContext<'_>, child: Widget| {
            seen.borrow_mut().push(("theme", snapshot(context)));
            child
        })
    }));
    builder.add_wrapper(move |child| {
        let seen = wrapper_seen.clone();
        Builder::new(move |context| {
            seen.borrow_mut().push(("wrapper", snapshot(context)));
            child.clone()
        })
        .into()
    });
    let mut tree = WidgetTree::new();
    let environment = RuntimeEnvironment {
        viewport: Size::new(500., 300.),
        safe_insets: EdgeInsets::only(0., 12., 0., 0.),
        ..Default::default()
    };
    tree.set_environment(environment.clone());
    tree.mount(builder.build().compose(leaf().with_key(key.clone())))
        .unwrap();
    let constraints = Constraints::loose(Size::new(500., 300.));
    tree.layout(constraints).unwrap();
    let leaf_id = tree.element_with_key(&key).unwrap();
    assert_eq!(
        tree.element_geometry(leaf_id).unwrap().1,
        Size::new(40., 60.)
    );

    let expected = |top| {
        (
            builder.size.unwrap(),
            1.75,
            Brightness::Dark,
            top,
            "es".to_owned(),
        )
    };
    assert_eq!(
        *seen.borrow(),
        [("theme", expected(12.)), ("wrapper", expected(12.))]
    );
    tree.set_environment(RuntimeEnvironment {
        safe_insets: EdgeInsets::only(0., 20., 0., 0.),
        ..environment
    });
    tree.layout(constraints).unwrap();
    assert_eq!(tree.element_with_key(&key), Some(leaf_id));
    let seen = seen.borrow();
    assert!(seen[2..].contains(&("theme", expected(20.))));
    assert!(seen[2..].contains(&("wrapper", expected(20.))));
}

#[test]
fn composed_preview_keeps_outer_media_query_for_unspecified_aspects() {
    use incular_config::{Brightness, EdgeInsets, RuntimeEnvironment};
    use incular_widgets::MediaQuery;

    let mut config = PreviewBuilder::new();
    config.text_scale_factor = Some(2.);
    let child = Builder::new(|context| {
        assert_eq!(context.media_query_size(), Some(Size::new(320., 240.)));
        assert_eq!(context.media_query_device_pixel_ratio(), Some(2.));
        assert_eq!(
            context.media_query_platform_brightness(),
            Some(Brightness::Dark)
        );
        assert_eq!(context.media_query_view_padding().unwrap().bottom, 8.);
        assert_eq!(context.media_query_text_scale(), Some(2.));
        leaf()
    });
    let mut tree = WidgetTree::new();
    tree.mount(
        MediaQuery::new(
            RuntimeEnvironment {
                viewport: Size::new(320., 240.),
                scale_factor: 2.,
                brightness: Brightness::Dark,
                view_padding: EdgeInsets::only(0., 0., 0., 8.),
                ..Default::default()
            },
            config.build().compose(child),
        )
        .into(),
    )
    .unwrap();
    tree.layout(Constraints::loose(Size::new(500., 300.)))
        .unwrap();
}

#[test]
fn multi_preview_transforms_members_before_its_shared_outer_wrapper() {
    use incular_widgets::widget_previews::{MultiPreview, Preview, PreviewTransform};

    struct Member {
        label: &'static str,
        brightness: Brightness,
        trace: Rc<RefCell<Vec<&'static str>>>,
    }

    impl PreviewTransform for Member {
        fn transform(&self) -> Preview {
            self.trace.borrow_mut().push(self.label);
            let mut builder = PreviewBuilder::new();
            builder.name = Some(self.label.to_owned());
            builder.brightness = Some(self.brightness);
            builder.add_wrapper(|child| ColoredBox::new(INNER, child).into());
            builder.build()
        }
    }

    struct Pair([Member; 2]);

    impl MultiPreview for Pair {
        fn previews(&self) -> Vec<&dyn PreviewTransform> {
            self.0
                .iter()
                .map(|item| item as &dyn PreviewTransform)
                .collect()
        }

        fn transform(&self) -> Vec<Preview> {
            self.transform_members()
                .into_iter()
                .map(|preview| {
                    let mut builder = preview.to_builder();
                    builder.name = Some(format!("Pair {}", preview.name().unwrap()));
                    builder.add_wrapper(|child| ColoredBox::new(OUTER, child).into());
                    builder.build()
                })
                .collect()
        }
    }

    let trace = Rc::new(RefCell::new(Vec::new()));
    let pair = Pair([
        Member {
            label: "Light",
            brightness: Brightness::Light,
            trace: trace.clone(),
        },
        Member {
            label: "Dark",
            brightness: Brightness::Dark,
            trace: trace.clone(),
        },
    ]);
    let result = pair.transform();
    assert_eq!(*trace.borrow(), ["Light", "Dark"]);
    assert_eq!(result.len(), 2);
    for (preview, label, brightness) in [
        (&result[0], "Pair Light", Brightness::Light),
        (&result[1], "Pair Dark", Brightness::Dark),
    ] {
        assert_eq!(preview.name(), Some(label));
        assert_eq!(preview.brightness(), Some(brightness));
        assert_eq!(painted_colors(preview.wrap(leaf())), [OUTER, INNER, LEAF]);
    }

    let fresh = pair.transform_members();
    assert_eq!(*trace.borrow(), ["Light", "Dark", "Light", "Dark"]);
    assert_eq!(fresh[0].name(), Some("Light"));
    assert_eq!(painted_colors(fresh[0].wrap(leaf())), [INNER, LEAF]);
    assert_eq!(result[0].name(), Some("Pair Light"));
}
