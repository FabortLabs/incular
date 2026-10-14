//! Text-direction contract, ported from Flutter 3.41.7.
//!
//! Flutter has no ambient default text direction. `Directionality.of` fails
//! when no ancestor exists (always), and `Row`/`Wrap` assert in debug builds
//! when their layout order depends on a direction nobody supplied. The first
//! test is the canonical Flutter "hello world without MaterialApp"
//! (`runApp(Center(child: Column(...)))`) translated one-to-one.
//!
//! Flutter 3.41.7 references for the rules below:
//! * `C:\\Manual Installs\\flutter\\packages\\flutter\\lib\\src\\rendering\\flex.dart`,
//!   `RenderFlex._debugHasNecessaryDirections` (lines 579-620).
//! * `C:\\Manual Installs\\flutter\\packages\\flutter\\lib\\src\\rendering\\wrap.dart`,
//!   `RenderWrap._debugHasNecessaryDirections` (lines 454-500).
//! * `C:\\Manual Installs\\flutter\\packages\\flutter\\lib\\src\\widgets\\banner.dart`,
//!   `_BannerState.build` (lines 297-315).
//! * `C:\\Manual Installs\\flutter\\packages\\flutter\\lib\\src\\widgets\\editable_text.dart`,
//!   `EditableTextState._textDirection` (line 4945).
//! * `C:\\Manual Installs\\flutter\\packages\\flutter\\lib\\src\\widgets\\basic.dart`,
//!   `Directionality.updateShouldNotify` (line 222).
//!
//! The debug-only direction assertions are intentionally tested under
//! `cfg(debug_assertions)`. Flutter's release fallback is documented by
//! `RenderFlex._flipMainAxis` (`rendering/flex.dart:855-866`, where null is
//! treated as LTR) and `RenderWrap._areAxesFlipped` (`rendering/wrap.dart:601-613`,
//! `textDirection ?? TextDirection.ltr`); this debug-profile test must not fake
//! release mode or weaken the strict build contract to exercise that fallback.

use incular::prelude::*;
use incular_config::Constraints;
use incular_core::{Color, Offset, Size};
use incular_rendering::PaintCommand;
use incular_widgets::internal::{GeneratedChildIdentity, TreeError, WidgetTree};

fn glyph_bounds(list: &incular_rendering::DisplayList) -> (f32, f32) {
    let mut stack = vec![Offset::ZERO];
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for command in list.commands() {
        match command {
            PaintCommand::PushTransform { transform } => {
                let top = *stack.last().expect("transform stack");
                stack.push(top + transform.translation_offset());
            }
            PaintCommand::PopTransform => {
                stack.pop();
            }
            PaintCommand::GlyphRun { run, .. } => {
                let shift = *stack.last().expect("transform stack");
                for glyph in run.glyphs.iter() {
                    let left = shift.x + glyph.offset.x;
                    min = min.min(left);
                    max = max.max(left + glyph.advance);
                }
            }
            _ => {}
        }
    }
    assert!(
        min.is_finite() && max.is_finite(),
        "Flutter 3.41.7 packages/flutter/lib/src/widgets/editable_text.dart:4945 must produce painted EditableText glyphs"
    );
    (min, max)
}

fn constraints() -> Constraints {
    Constraints::loose(Size::new(480., 320.))
}

fn style() -> TextStyle {
    TextStyle::new().font_size(20.).color(Color::BLACK)
}

fn box_10() -> Widget {
    SizedBox::new().width(10.).height(10.).into()
}

fn run(root: impl Fn() -> Widget + 'static) -> Result<(), TreeError> {
    let app = Application::new(move |_| root())?;
    let mut runtime = app.into_runtime();
    runtime.run_frame(constraints()).map(|_| ())
}

fn assert_missing_directionality(
    result: Result<(), TreeError>,
    widget: &str,
    flutter_source: &str,
) {
    match result {
        Err(TreeError::MissingDirectionality { widget: found, .. }) => assert_eq!(
            found, widget,
            "Flutter 3.41.7 {flutter_source} requires Directionality for {widget}"
        ),
        other => panic!(
            "Flutter 3.41.7 {flutter_source} requires a missing-direction error for {widget}; got {other:?}"
        ),
    }
}

/// Flutter:
///
/// ```dart
/// runApp(const Center(child: Column(
///   mainAxisAlignment: MainAxisAlignment.center,
///   children: [
///     Directionality(textDirection: TextDirection.ltr,
///         child: Text('Hello, World! (Directionality)', style: ...)),
///     Text('Hello, World! (Text Property)',
///         textDirection: TextDirection.ltr, style: ...),
///   ])));
/// ```
#[test]
fn flutter_hello_world_without_an_app_shell_builds() {
    let result = run(|| {
        Center::new(
            Column::new([
                Widget::from(Directionality::new(
                    TextDirection::Ltr,
                    Text::new("Hello, World! (Directionality)").style(style()),
                )),
                Widget::from(
                    Text::new("Hello, World! (Text Property)")
                        .text_direction(TextDirection::Ltr)
                        .style(style()),
                ),
            ])
            .main_axis_alignment(MainAxisAlignment::Center),
        )
        .into()
    });
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn text_without_any_direction_is_a_build_error() {
    let result = run(|| Text::new("no direction anywhere").into());
    assert_missing_directionality(
        result,
        "Text",
        "widgets/text.dart RichText.createRenderObject / Directionality.of",
    );
}

#[test]
fn text_inside_directionality_builds() {
    let result = run(|| {
        Directionality::new(
            TextDirection::Rtl,
            Text::new("\u{5e9}\u{5dc}\u{5d5}\u{5dd}"),
        )
        .into()
    });
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn banner_requires_both_directions_or_an_ancestor() {
    let missing = run(|| Banner::new("DEBUG", BannerLocation::TopStart).into());
    assert_missing_directionality(
        missing,
        "Banner",
        "widgets/banner.dart _BannerState.build (Directionality.of at lines 306 and 308)",
    );

    let missing_layout = run(|| {
        Banner::new("DEBUG", BannerLocation::TopStart)
            .text_direction(TextDirection::Ltr)
            .into()
    });
    assert_missing_directionality(
        missing_layout,
        "Banner",
        "widgets/banner.dart _BannerState.build (both textDirection and layoutDirection are required at lines 297-308)",
    );

    let explicit = run(|| {
        Banner::new("DEBUG", BannerLocation::TopStart)
            .text_direction(TextDirection::Ltr)
            .layout_direction(TextDirection::Rtl)
            .into()
    });
    assert!(
        explicit.is_ok(),
        "Flutter 3.41.7 widgets/banner.dart _BannerState.build accepts both explicit directions: {explicit:?}"
    );

    let inherited = run(|| {
        Directionality::new(
            TextDirection::Rtl,
            Banner::new("DEBUG", BannerLocation::TopStart),
        )
        .into()
    });
    assert!(
        inherited.is_ok(),
        "Flutter 3.41.7 widgets/banner.dart _BannerState.build resolves both directions from Directionality.of: {inherited:?}"
    );
}

#[test]
fn editable_text_requires_explicit_or_ambient_direction() {
    let missing = run(|| EditableText::new(TextEditingController::new()).into());
    assert_missing_directionality(
        missing,
        "EditableText",
        "widgets/editable_text.dart EditableTextState._textDirection (line 4945)",
    );

    let explicit = run(|| {
        EditableText::new(TextEditingController::new())
            .text_direction(TextDirection::Rtl)
            .into()
    });
    assert!(
        explicit.is_ok(),
        "Flutter 3.41.7 widgets/editable_text.dart EditableTextState._textDirection accepts an explicit direction: {explicit:?}"
    );

    let inherited = run(|| {
        Directionality::new(
            TextDirection::Ltr,
            EditableText::new(TextEditingController::new()),
        )
        .into()
    });
    assert!(
        inherited.is_ok(),
        "Flutter 3.41.7 widgets/editable_text.dart EditableTextState._textDirection resolves Directionality.of: {inherited:?}"
    );
}

#[test]
fn editable_text_directionality_case_checks_physical_start_alignment() {
    let mut tree = WidgetTree::new();
    tree.mount(
        Directionality::new(
            TextDirection::Rtl,
            EditableText::new(TextEditingController::with_text("hi"))
                .text_align(TextAlign::Start)
                .size(Size::new(200., 32.)),
        )
        .into(),
    )
    .expect("mount ambient RTL EditableText");
    tree.layout(Constraints::tight(Size::new(200., 32.)))
        .expect("layout ambient RTL EditableText");
    let (_, max) = glyph_bounds(&tree.paint());
    assert!(
        (max - 192.).abs() < 1.5,
        "Flutter 3.41.7 packages/flutter/lib/src/widgets/editable_text.dart:4945 resolves Directionality.of(context), and packages/flutter/lib/src/painting/text_painter.dart:856-859 places TextAlign.start at the RTL physical right edge: {max}"
    );
}

#[test]
fn updates_reject_directionless_text_editable_text_and_banner() {
    let mut text_tree = WidgetTree::new();
    let text_root = text_tree
        .mount(
            Text::new("initial")
                .text_direction(TextDirection::Ltr)
                .into(),
        )
        .expect("mount explicitly directed Text");
    assert_missing_directionality(
        text_tree.update(text_root, Text::new("replacement").into()),
        "Text",
        "widgets/text.dart RichText.createRenderObject / Directionality.of during update",
    );

    let mut editable_tree = WidgetTree::new();
    let editable_root = editable_tree
        .mount(
            EditableText::new(TextEditingController::new())
                .text_direction(TextDirection::Ltr)
                .into(),
        )
        .expect("mount explicitly directed EditableText");
    assert_missing_directionality(
        editable_tree.update(
            editable_root,
            EditableText::new(TextEditingController::new()).into(),
        ),
        "EditableText",
        "widgets/editable_text.dart EditableTextState._textDirection (line 4945) during update",
    );

    let mut banner_tree = WidgetTree::new();
    let banner_root = banner_tree
        .mount(
            Banner::new("DEBUG", BannerLocation::TopStart)
                .text_direction(TextDirection::Ltr)
                .layout_direction(TextDirection::Ltr)
                .into(),
        )
        .expect("mount explicitly directed Banner");
    assert_missing_directionality(
        banner_tree.update(
            banner_root,
            Banner::new("DEBUG", BannerLocation::TopStart).into(),
        ),
        "Banner",
        "widgets/banner.dart _BannerState.build (Directionality.of at lines 306 and 308) during update",
    );
}

#[test]
fn center_and_default_column_need_no_direction() {
    // Align(Alignment.center) is not directional; a vertical flex with the
    // default crossAxisAlignment.center is not either (flex.dart
    // `_debugHasNecessaryDirections`, basic.dart `_needTextDirection`).
    let result = run(|| {
        Center::new(Column::new([
            Widget::from(SizedBox::new().width(10.).height(10.)),
            Widget::from(SizedBox::new().width(10.).height(10.)),
        ]))
        .into()
    });
    assert!(result.is_ok(), "{result:?}");
}

#[cfg(debug_assertions)]
#[test]
fn horizontal_flex_without_direction_asserts_in_debug_builds() {
    let result = run(|| {
        Row::new([
            Widget::from(SizedBox::new().width(10.).height(10.)),
            Widget::from(SizedBox::new().width(10.).height(10.)),
        ])
        .into()
    });
    assert_missing_directionality(
        result,
        "Flex",
        "rendering/flex.dart RenderFlex._debugHasNecessaryDirections",
    );
}

#[cfg(debug_assertions)]
#[test]
fn vertical_flex_with_start_and_end_cross_axis_asserts_in_debug_builds() {
    for alignment in [CrossAxisAlignment::Start, CrossAxisAlignment::End] {
        let result = run(move || {
            Flex::new(
                Axis::Vertical,
                [Widget::from(SizedBox::new().width(10.).height(10.))],
            )
            .cross_axis_alignment(alignment)
            .into()
        });
        assert_missing_directionality(
            result,
            "Flex",
            "rendering/flex.dart RenderFlex._debugHasNecessaryDirections (vertical crossAxisAlignment start/end)",
        );
    }
}

#[test]
fn explicit_flex_direction_or_ancestor_directionality_satisfies_row() {
    let explicit = run(|| {
        Row::new([
            Widget::from(SizedBox::new().width(10.).height(10.)),
            Widget::from(SizedBox::new().width(10.).height(10.)),
        ])
        .text_direction(TextDirection::Rtl)
        .into()
    });
    assert!(explicit.is_ok(), "{explicit:?}");

    let inherited = run(|| {
        Directionality::new(
            TextDirection::Rtl,
            Row::new([
                Widget::from(SizedBox::new().width(10.).height(10.)),
                Widget::from(SizedBox::new().width(10.).height(10.)),
            ]),
        )
        .into()
    });
    assert!(inherited.is_ok(), "{inherited:?}");
}

#[cfg(debug_assertions)]
#[test]
fn wrap_start_and_end_rules_require_direction_in_debug_builds() {
    let horizontal_start = run(|| {
        Wrap::new([box_10()])
            .direction(Axis::Horizontal)
            .alignment(WrapAlignment::Start)
            .into()
    });
    assert_missing_directionality(
        horizontal_start,
        "Wrap",
        "rendering/wrap.dart RenderWrap._debugHasNecessaryDirections (horizontal alignment start)",
    );

    let horizontal_end = run(|| {
        Wrap::new([box_10()])
            .direction(Axis::Horizontal)
            .alignment(WrapAlignment::End)
            .into()
    });
    assert_missing_directionality(
        horizontal_end,
        "Wrap",
        "rendering/wrap.dart RenderWrap._debugHasNecessaryDirections (horizontal alignment end)",
    );

    let vertical_run_start = run(|| {
        Wrap::new([box_10()])
            .direction(Axis::Vertical)
            .run_alignment(WrapAlignment::Start)
            .into()
    });
    assert_missing_directionality(
        vertical_run_start,
        "Wrap",
        "rendering/wrap.dart RenderWrap._debugHasNecessaryDirections (vertical runAlignment start)",
    );

    let vertical_run_end = run(|| {
        Wrap::new([box_10()])
            .direction(Axis::Vertical)
            .run_alignment(WrapAlignment::End)
            .into()
    });
    assert_missing_directionality(
        vertical_run_end,
        "Wrap",
        "rendering/wrap.dart RenderWrap._debugHasNecessaryDirections (vertical runAlignment end)",
    );

    let vertical_cross_start = run(|| {
        Wrap::new([box_10()])
            .direction(Axis::Vertical)
            .cross_axis_alignment(WrapCrossAlignment::Start)
            .into()
    });
    assert_missing_directionality(
        vertical_cross_start,
        "Wrap",
        "rendering/wrap.dart RenderWrap._debugHasNecessaryDirections (vertical crossAxisAlignment start)",
    );

    let vertical_cross_end = run(|| {
        Wrap::new([box_10()])
            .direction(Axis::Vertical)
            .cross_axis_alignment(WrapCrossAlignment::End)
            .into()
    });
    assert_missing_directionality(
        vertical_cross_end,
        "Wrap",
        "rendering/wrap.dart RenderWrap._debugHasNecessaryDirections (vertical crossAxisAlignment end)",
    );

    let explicit_horizontal = run(|| {
        Wrap::new([box_10()])
            .alignment(WrapAlignment::End)
            .text_direction(TextDirection::Rtl)
            .into()
    });
    assert!(
        explicit_horizontal.is_ok(),
        "Flutter 3.41.7 rendering/wrap.dart RenderWrap._debugHasNecessaryDirections accepts explicit textDirection: {explicit_horizontal:?}"
    );

    let explicit_vertical = run(|| {
        Wrap::new([box_10()])
            .direction(Axis::Vertical)
            .run_alignment(WrapAlignment::End)
            .cross_axis_alignment(WrapCrossAlignment::End)
            .text_direction(TextDirection::Ltr)
            .into()
    });
    assert!(
        explicit_vertical.is_ok(),
        "Flutter 3.41.7 rendering/wrap.dart RenderWrap._debugHasNecessaryDirections accepts explicit textDirection: {explicit_vertical:?}"
    );
}

#[test]
fn context_reports_no_direction_without_an_ancestor() {
    use std::{cell::Cell, rc::Rc};
    let seen = Rc::new(Cell::new(None));
    let probe = seen.clone();
    let result = run(move || {
        let probe = probe.clone();
        LayoutBuilder::new(move |context, _| {
            probe.set(Some(context.text_direction()));
            SizedBox::new().width(1.).height(1.).into()
        })
        .into()
    });
    assert!(result.is_ok(), "{result:?}");
    assert_eq!(seen.get(), Some(None), "no implicit left-to-right default");
}

#[test]
fn lazy_child_direction_errors_keep_the_generated_child_context() {
    let mut tree = WidgetTree::new();
    tree.mount(LayoutBuilder::new(|_, _| Widget::from(Text::new("late child"))).into())
        .expect("the lazy owner itself has no direction requirement");

    let error = tree
        .layout(Constraints::tight(Size::new(100., 20.)))
        .expect_err("the lazily generated Text must require Directionality");
    match error {
        TreeError::InvalidGeneratedChild {
            child: GeneratedChildIdentity::LayoutBuilder,
            source,
            ..
        } => match *source {
            TreeError::MissingDirectionality { widget, .. } => assert_eq!(
                widget, "Text",
                "Flutter 3.41.7 widgets/text.dart RichText resolves direction with Directionality.of; reconciliation.rs::reconcile_dynamic_children must preserve the source error"
            ),
            source => panic!(
                "Flutter 3.41.7 widgets/text.dart RichText must be the generated-child source; got {source:?}"
            ),
        },
        other => panic!(
            "Flutter 3.41.7 lazy child direction errors must surface through reconciliation.rs::reconcile_dynamic_children as InvalidGeneratedChild; got {other:?}"
        ),
    }
}

#[test]
fn changing_directionality_relayouts_descendant_flex() {
    let constraints = Constraints::tight(Size::new(100., 20.));
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(Directionality::new(TextDirection::Ltr, Row::new([box_10(), box_10()])).into())
        .expect("mount Directionality");
    tree.layout(constraints)
        .expect("initial LTR layout must complete");

    let row = tree
        .children(root)
        .and_then(|children| children.first().copied())
        .expect("Directionality materializes its row child");
    let children = tree.children(row).expect("row children").to_vec();
    let ltr_first = tree
        .element_bounds(children[0])
        .expect("first LTR child bounds")
        .origin
        .x;
    let ltr_second = tree
        .element_bounds(children[1])
        .expect("second LTR child bounds")
        .origin
        .x;
    assert!(
        ltr_first < ltr_second,
        "Flutter 3.41.7 rendering/flex.dart RenderFlex._flipMainAxis lays out LTR children in declaration order"
    );

    tree.update(
        root,
        Directionality::new(TextDirection::Rtl, Row::new([box_10(), box_10()])).into(),
    )
    .expect("update Directionality");
    tree.layout(constraints)
        .expect("updated RTL layout must complete");

    let updated_row = tree
        .children(root)
        .and_then(|children| children.first().copied())
        .expect("updated Directionality materializes its row child");
    let updated_children = tree
        .children(updated_row)
        .expect("updated row children")
        .to_vec();
    let rtl_first = tree
        .element_bounds(updated_children[0])
        .expect("first RTL child bounds")
        .origin
        .x;
    let rtl_second = tree
        .element_bounds(updated_children[1])
        .expect("second RTL child bounds")
        .origin
        .x;
    assert!(
        rtl_first > rtl_second,
        "Flutter 3.41.7 widgets/basic.dart Directionality.updateShouldNotify (line 222) must re-layout descendants when textDirection changes"
    );
    // Flutter 3.41.7 rendering/flex.dart RenderFlex._flipMainAxis (rtl => true)
    // and RenderFlex._distributeSpace (MainAxisAlignment.start, flipped =>
    // leading space = remaining space, line 236). Two 10px children in a 100px
    // row leave 80px free, and that space leads on the right. The last child
    // starts at x = 80 and the first child at x = 90, so the first child sits at
    // the right edge. The LTR positions are 0 (first) and 10 (second).
    //
    // The first child's LTR position (0) equals neither RTL position (90 or 80).
    // With MainAxisAlignment.start, Flutter does not mirror the LTR offsets onto
    // the RTL children (the children are not swapped into each other's slots),
    // so the RTL positions are compared to their own derived values.
    assert_eq!(
        ltr_first, 0.,
        "Flutter LTR: first child at the leading edge"
    );
    assert_eq!(ltr_second, 10., "Flutter LTR: second child after the first");
    assert_eq!(
        rtl_first, 90.,
        "Flutter RTL: first child at the right edge (100 - 10)"
    );
    assert_eq!(
        rtl_second, 80.,
        "Flutter RTL: second child packed after the first (90 - 10)"
    );
}
