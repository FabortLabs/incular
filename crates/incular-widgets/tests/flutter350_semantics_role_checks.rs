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
