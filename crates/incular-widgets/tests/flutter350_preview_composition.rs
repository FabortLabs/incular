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
