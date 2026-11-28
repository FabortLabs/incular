#![cfg(debug_assertions)]

use incular_config::{Clip, Constraints};
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{Brush, DisplayList, PaintCommand, TileMode};
use incular_widgets::{Center, Column, SizedBox, UnconstrainedBox, Widget, internal::WidgetTree};

const VIEW: Size = Size {
    width: 800.,
    height: 600.,
};
const CHILD_COLOR: Color = Color::rgba(23, 42, 170, 255);

fn scene(flex: bool, height: Option<f32>, clip: Clip, colored: bool) -> Widget {
    let child = if colored {
        Widget::box_(Size::new(200., 200.), CHILD_COLOR)
    } else {
        SizedBox::square(200.).into()
    };
    let body: Widget = if flex {
        Column::new([child]).clip_behavior(clip).into()
    } else {
        UnconstrainedBox::new(child).clip_behavior(clip).into()
    };
    Center::new(if let Some(height) = height {
        Widget::from(SizedBox::new().height(height).child(body))
    } else {
        // Preserve the same wrapper and child identity across rebuilds.
        Widget::from(SizedBox::new().child(body))
    })
    .into()
}

struct Host {
    tree: WidgetTree,
}

impl Host {
    fn new(widget: Widget) -> Self {
        let mut tree = WidgetTree::new();
        tree.mount(widget).unwrap();
        Self { tree }
    }

    fn update(&mut self, widget: Widget) {
        self.tree.update(self.tree.root().unwrap(), widget).unwrap();
    }

    fn paint(&mut self) -> DisplayList {
        self.tree.layout(Constraints::tight(VIEW)).unwrap();
        self.tree
            .update_compositor(std::time::Instant::now())
            .unwrap();
        let display = self.tree.paint();
        self.tree.verify_invariants().unwrap();
        display
    }
}

fn marker(rect: Rect) -> (Offset, Size) {
    (rect.origin, rect.size)
}

fn markers(list: &DisplayList) -> Vec<(Offset, Size)> {
    list.commands()
        .iter()
        .filter_map(|command| {
            if let PaintCommand::RRect {
                rrect,
                brush: Brush::LinearGradient(gradient),
            } = command
            {
                assert_eq!(gradient.start, Offset::ZERO);
                assert_eq!(gradient.end, Offset::new(10., 10.));
                assert_eq!(gradient.tile_mode, TileMode::Repeated);
                assert_eq!(
                    gradient
                        .stops
                        .as_slice()
                        .iter()
                        .filter(|stop| stop.offset == 0.25)
                        .count(),
                    2
                );
                assert_eq!(
                    gradient
                        .stops
                        .as_slice()
                        .iter()
                        .filter(|stop| stop.offset == 0.75)
                        .count(),
                    2
                );
                assert!(
                    gradient
                        .stops
                        .as_slice()
                        .iter()
                        .all(|stop| stop.color.alpha == 191)
                );
                Some(marker(rrect.rect))
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn a_fitting_constraints_transform_paints_no_indicator_or_diagnostic() {
    let mut host = Host::new(scene(false, None, Clip::None, false));
    assert!(markers(&host.paint()).is_empty());
    assert!(host.tree.take_constraint_overflow_diagnostics().is_empty());
}

#[test]
fn constraint_overflow_paints_both_edges_but_reports_only_once() {
    let view = || scene(false, Some(100.), Clip::None, false);
    let mut host = Host::new(view());
    let expected = [
        (Offset::ZERO, Size::new(200., 10.)),
        (Offset::new(0., 90.), Size::new(200., 10.)),
    ];
    assert_eq!(markers(&host.paint()), expected);
    let reports = host.tree.take_constraint_overflow_diagnostics();
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].level(), incular_core::DiagnosticLevel::Summary);
    assert_eq!(
        reports[0].to_string(),
        "A RenderConstraintsTransformBox overflowed by 50 pixels on the top and 50 pixels on the bottom."
    );

    host.update(view());
    assert_eq!(markers(&host.paint()), expected);
    assert!(host.tree.take_constraint_overflow_diagnostics().is_empty());
}

#[test]
fn zero_sized_overflow_paints_no_indicator_or_report() {
    let mut host = Host::new(scene(false, Some(0.), Clip::None, false));
    assert!(markers(&host.paint()).is_empty());
    assert!(host.tree.take_constraint_overflow_diagnostics().is_empty());
}

#[test]
fn flex_indicators_appear_only_for_nonzero_overflowing_boxes() {
    let view = |height| scene(true, height, Clip::None, false);
    let mut host = Host::new(view(None));
    assert!(markers(&host.paint()).is_empty());
    assert!(host.tree.take_flex_overflow_diagnostics().is_empty());

    host.update(view(Some(100.)));
    assert_eq!(
        markers(&host.paint()),
        [(Offset::new(0., 90.), Size::new(200., 10.))]
    );
    let reports = host.tree.take_flex_overflow_diagnostics();
    assert_eq!(reports.len(), 1);
    assert_eq!(
        reports[0].to_string(),
        "A RenderFlex overflowed by 100 pixels on the bottom."
    );

    host.update(view(Some(0.)));
    assert!(markers(&host.paint()).is_empty());
    assert!(host.tree.take_flex_overflow_diagnostics().is_empty());
}

#[test]
fn constraint_indicators_are_above_children_and_disappear_when_clipped() {
    let mut host = Host::new(scene(false, Some(100.), Clip::None, true));
    let list = host.paint();
    let commands = list.commands();
    let child = commands
        .iter()
        .position(|op| matches!(op, PaintCommand::Rect { color, .. } if *color == CHILD_COLOR))
        .unwrap();
    let stripe = commands
        .iter()
        .position(|op| {
            matches!(
                op,
                PaintCommand::RRect {
                    brush: Brush::LinearGradient(_),
                    ..
                }
            )
        })
        .unwrap();
    assert!(child < stripe);
    assert!(
        commands[stripe..]
            .iter()
            .any(|op| matches!(op, PaintCommand::GlyphRun { .. }))
    );
    host.update(scene(false, Some(100.), Clip::HardEdge, true));
    assert!(markers(&host.paint()).is_empty());
}

#[test]
fn an_existing_indicator_clears_after_only_the_child_shrinks() {
    let view = |height| {
        Center::new(SizedBox::new().height(100.).child(UnconstrainedBox::new(
            SizedBox::new().width(200.).height(height),
        )))
        .into()
    };
    let mut host = Host::new(view(200.));
    assert_eq!(markers(&host.paint()).len(), 2);
    assert_eq!(host.tree.take_constraint_overflow_diagnostics().len(), 1);
    host.update(view(80.));
    assert!(markers(&host.paint()).is_empty());
    host.update(view(200.));
    assert_eq!(markers(&host.paint()).len(), 2);
    assert!(host.tree.take_constraint_overflow_diagnostics().is_empty());
}

#[test]
fn unconstrained_square_warns_only_for_the_default_and_explicit_none() {
    for setting in [
        None,
        Some(Clip::None),
        Some(Clip::HardEdge),
        Some(Clip::AntiAlias),
        Some(Clip::AntiAliasWithSaveLayer),
    ] {
        let proxy = UnconstrainedBox::new(SizedBox::square(400.));
        let mut host = Host::new(
            Center::new(SizedBox::square(200.).child(match setting {
                None => proxy,
                Some(clip) => proxy.clip_behavior(clip),
            }))
            .into(),
        );
        host.paint();
        let reports = host.tree.take_constraint_overflow_diagnostics();
        if matches!(setting, None | Some(Clip::None)) {
            assert_eq!(reports.len(), 1, "setting {setting:?}");
            let [report] = reports.as_slice() else {
                unreachable!()
            };
            assert_eq!(report.level(), incular_core::DiagnosticLevel::Summary);
            assert!(
                report
                    .to_string()
                    .starts_with("A RenderConstraintsTransformBox overflowed")
            );
            assert_eq!(
                [report.left, report.top, report.right, report.bottom],
                [100.; 4]
            );
        } else {
            assert!(reports.is_empty(), "setting {setting:?}");
        }
    }
}
