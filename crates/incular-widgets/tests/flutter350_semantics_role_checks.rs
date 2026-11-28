//! Debug role diagnostics after widget semantics have been merged and attached.
#![cfg(debug_assertions)]

use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_semantics::{Role, SemanticRoleIssueKind as Issue};
use incular_widgets::{
    Column, Directionality, ExcludeSemantics, Semantics, SizedBox, Text, Widget,
    internal::WidgetTree,
};

fn label() -> Text {
    Text::new("some child").text_direction(TextDirection::Ltr)
}

fn mount(content: impl Into<Widget>) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(Directionality::new(TextDirection::Ltr, content).into())
        .unwrap();
    pump(&mut tree);
    tree
}

fn pump(tree: &mut WidgetTree) {
    tree.layout(Constraints::tight(Size::new(200., 120.)))
        .unwrap();
    tree.update_semantics();
}

fn issues(widget: impl Into<Widget>) -> Vec<(Role, Issue)> {
    let tree = mount(widget);
    tree.debug_semantic_role_issues()
        .iter()
        .map(|issue| {
            assert_eq!(tree.semantics().node(issue.node).unwrap().role, issue.role);
            (issue.role, issue.kind)
        })
        .collect()
}

#[test]
fn tab_requires_selected_state_then_tap_when_enabled() {
    assert_eq!(
        issues(Semantics::new(label()).role(Role::Tab)),
        [(Role::Tab, Issue::TabWithoutSelectedState)]
    );
    assert_eq!(
        issues(Semantics::new(label()).role(Role::Tab).selected(false)),
        [(Role::Tab, Issue::EnabledTabWithoutTap)]
    );
    assert!(
        issues(
            Semantics::new(label())
                .role(Role::Tab)
                .selected(false)
                .on_tap(|| {})
        )
        .is_empty()
    );
    assert!(
        issues(
            Semantics::new(label())
                .role(Role::Tab)
                .selected(false)
                .enabled(false)
        )
        .is_empty()
    );
    assert!(
        issues(
            Semantics::new(label())
                .role(Role::Tab)
                .selected(false)
                .block_user_actions(true)
        )
        .is_empty()
    );
}

#[test]
fn list_item_is_checked_against_its_semantic_parent_not_layout_widgets() {
    assert_eq!(
        issues(Semantics::new(label()).role(Role::ListItem)),
        [(Role::ListItem, Issue::ListItemOutsideList)]
    );
    let item = || Semantics::new(label()).role(Role::ListItem);
    assert!(
        issues(
            Semantics::new(Column::new([item()]))
                .role(Role::List)
                .explicit_child_nodes(true)
        )
        .is_empty()
    );
}

#[test]
fn tab_bar_rejects_hidden_or_wrong_role_children_but_accepts_an_actual_tab() {
    assert_eq!(
        issues(Semantics::new(ExcludeSemantics::new(label())).role(Role::TabList)),
        [(Role::TabList, Issue::EmptyTabList)]
    );
    assert_eq!(
        issues(
            Semantics::new(Semantics::new(label()))
                .role(Role::TabList)
                .explicit_child_nodes(true)
        ),
        [(Role::TabList, Issue::TabListWithNonTabChild)]
    );
    assert!(
        issues(
            Semantics::new(
                Semantics::new(label())
                    .role(Role::Tab)
                    .selected(false)
                    .on_tap(|| {}),
            )
            .role(Role::TabList)
            .explicit_child_nodes(true)
        )
        .is_empty()
    );
}

#[test]
fn menus_require_content_and_menu_items_require_a_menu_ancestor() {
    let excluded = || ExcludeSemantics::new(label());
    assert_eq!(
        issues(Semantics::new(excluded()).role(Role::Menu)),
        [(Role::Menu, Issue::EmptyMenu)]
    );
    assert_eq!(
        issues(Semantics::new(excluded()).role(Role::MenuItem)),
        [(Role::MenuItem, Issue::MenuItemOutsideMenu)]
    );
    let item = || Semantics::new(label()).role(Role::MenuItem).on_tap(|| {});
    assert!(
        issues(
            Semantics::new(item().selected(false))
                .role(Role::Menu)
                .explicit_child_nodes(true)
        )
        .is_empty()
    );
    assert!(
        issues(
            Semantics::new(
                Semantics::new(item())
                    .container(true)
                    .explicit_child_nodes(true),
            )
            .role(Role::Menu)
            .explicit_child_nodes(true)
        )
        .is_empty()
    );
}

#[test]
fn alert_and_status_roles_cannot_double_announce_as_explicit_live_regions() {
    for role in [Role::Alert, Role::Status] {
        assert_eq!(
            issues(
                Semantics::new(SizedBox::square(1.))
                    .role(role)
                    .live_region(true)
            ),
            [(role, Issue::LiveAlertOrStatus)]
        );
    }
    assert!(
        issues(
            Semantics::new(Column::new(
                [Role::Status, Role::Alert]
                    .map(|role| { Semantics::new(SizedBox::square(1.)).role(role) })
            ))
            .explicit_child_nodes(true)
        )
        .is_empty()
    );
}

#[test]
fn replacing_a_bad_node_recomputes_the_report_without_leftover_errors() {
    let mut tree = mount(Semantics::new(label()).role(Role::Tab));
    assert_eq!(tree.debug_semantic_role_issues().len(), 1);
    tree.mount(
        Semantics::new(label())
            .role(Role::Tab)
            .selected(true)
            .on_tap(|| {})
            .into(),
    )
    .unwrap();
    pump(&mut tree);
    assert!(tree.debug_semantic_role_issues().is_empty());
}

type RangeCase = (
    usize,
    Option<&'static str>,
    Option<&'static str>,
    Option<&'static str>,
    Option<Issue>,
);

fn check_ranges(role: Role, cases: &[RangeCase]) {
    for &(line, value, min, max, expected) in cases {
        let mut widget = Semantics::new(SizedBox::shrink()).role(role);
        if let Some(value) = value {
            widget = widget.value(value);
        }
        if let Some(min) = min {
            widget = widget.min_value(min);
        }
        if let Some(max) = max {
            widget = widget.max_value(max);
        }
        assert_eq!(
            issues(widget),
            expected
                .into_iter()
                .map(|issue| (role, issue))
                .collect::<Vec<_>>(),
            "Flutter registration at line {line}"
        );
    }
}

#[test]
fn progress_bar_needs_value_and_bounds_and_validates_its_range() {
    use Issue::*;
    check_ranges(
        Role::ProgressBar,
        &[
            (
                1082,
                None,
                None,
                None,
                Some(ProgressBarMissingValueOrBounds),
            ),
            (
                1095,
                None,
                Some("0"),
                Some("10"),
                Some(ProgressBarMissingValueOrBounds),
            ),
            (
                1113,
                Some("5"),
                None,
                None,
                Some(ProgressBarMissingValueOrBounds),
            ),
            (
                1126,
                Some("invalid"),
                Some("0"),
                Some("10"),
                Some(InvalidRangeNumber),
            ),
            (
                1149,
                Some("invalid"),
                Some("0%"),
                Some("100%"),
                Some(InvalidRangeNumber),
            ),
            (
                1174,
                Some("invalid"),
                Some("invalid"),
                Some("invalid"),
                Some(InvalidRangeNumber),
            ),
            (
                1197,
                Some("5"),
                Some("10"),
                Some("0"),
                Some(InvertedValueRange),
            ),
            (
                1216,
                Some("20"),
                Some("0"),
                Some("10"),
                Some(ValueOutsideRange),
            ),
            (
                1238,
                Some("150%"),
                Some("0"),
                Some("10"),
                Some(PercentageOutsideRange),
            ),
            (1257, Some("5"), Some("0"), Some("10"), None),
            (1273, Some("50%"), Some("0"), Some("10"), None),
        ],
    );
}

#[test]
fn slider_needs_each_bound_and_compares_numbers_and_percentages_separately() {
    use Issue::*;
    check_ranges(
        Role::Slider,
        &[
            (1291, None, None, None, Some(SliderMissingValue)),
            (1304, Some("50"), None, None, Some(SliderMissingMinValue)),
            (
                1321,
                Some("50"),
                Some("0"),
                None,
                Some(SliderMissingMaxValue),
            ),
            (
                1339,
                Some("150"),
                Some("0"),
                Some("100"),
                Some(ValueOutsideRange),
            ),
            (
                1358,
                Some("50"),
                Some("100"),
                Some("0"),
                Some(InvertedValueRange),
            ),
            (
                1377,
                Some("not-a-number"),
                Some("0"),
                Some("100"),
                Some(InvalidRangeNumber),
            ),
            (1396, Some("50"), Some("0"), Some("100"), None),
            (1412, Some("50%"), Some("0"), Some("100"), None),
            (
                1428,
                Some("150%"),
                Some("0"),
                Some("100"),
                Some(PercentageOutsideRange),
            ),
        ],
    );
}

#[test]
fn textual_bounds_keep_the_original_annotation_and_project_valid_numeric_limits() {
    let tree = mount(
        Semantics::new(SizedBox::shrink())
            .role(Role::Slider)
            .value("45%")
            .min_value(" 1.5 ")
            .max_value("6.50"),
    );
    assert!(tree.debug_semantic_role_issues().is_empty());
    let node = tree
        .semantics()
        .node(tree.semantics().root().unwrap())
        .unwrap();
    assert_eq!(node.value.as_deref(), Some("45%"));
    assert_eq!(node.state.numeric_min, Some(1.5));
    assert_eq!(node.state.numeric_max, Some(6.50));
    let text = node.state.value_bounds_text.as_ref().unwrap();
    assert_eq!(text.min.as_deref(), Some(" 1.5 "));
    assert_eq!(text.max.as_deref(), Some("6.50"));

    let typed = mount(
        Semantics::new(SizedBox::shrink())
            .role(Role::ProgressBar)
            .numeric_range(2., 6.)
            .numeric_value(2.),
    );
    assert!(typed.debug_semantic_role_issues().is_empty());
}
