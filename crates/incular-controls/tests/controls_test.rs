//! Comprehensive test suite for incular-controls.

use incular_config::Constraints;
use incular_controls::prelude::*;
use incular_controls::{
    alert_dialog, avatar, checkbox, drawer, meter, progress, scroll_area, tabs, toast,
};
use incular_core::{Offset, Size};
use incular_rendering::{Brush, PaintCommand};
use incular_widgets::{
    Directionality, MouseCursor, TextDirection, Widget,
    internal::{ActionId, ButtonState, TextEditingController, WidgetTree},
};

/// Controls construct their own `Text`/`EditableText` descendants from
/// labels and placeholders. Mount those roots under the same explicit scope
/// an application shell would provide. Directionality builds its child lazily
/// during the first layout, so callers resolve that child afterwards.
fn mount_ltr(
    tree: &mut WidgetTree,
    widget: impl Into<Widget>,
) -> incular_widgets::internal::ElementId {
    tree.mount(Directionality::new(TextDirection::Ltr, widget).into())
        .expect("mount directional control root")
}

fn scoped_child(
    tree: &WidgetTree,
    scope: incular_widgets::internal::ElementId,
) -> incular_widgets::internal::ElementId {
    tree.children(scope)
        .and_then(|children| children.first().copied())
        .expect("directionality child after layout")
}

#[test]
fn control_theme_contrast_and_defaults() {
    let light = ControlTheme::light();
    let dark = ControlTheme::dark();

    assert_ne!(light.colors.background, dark.colors.background);
    assert_ne!(light.colors.foreground, dark.colors.foreground);
    assert_eq!(light.metrics.border_radius, 4.0);
    assert_eq!(light.density, ControlDensity::Standard);
}

#[test]
fn control_density_scaling() {
    let compact = ControlDensity::Compact;
    let standard = ControlDensity::Standard;
    let comfortable = ControlDensity::Comfortable;

    assert!(compact.control_height() < standard.control_height());
    assert!(standard.control_height() < comfortable.control_height());
}

#[test]
fn composite_navigation_starts_at_first_enabled_and_supports_typeahead() {
    let mut controller = CompositeController::new().loop_navigation(false);
    controller.set_items([
        CompositeItem {
            id: "Disabled".into(),
            disabled: true,
        },
        CompositeItem {
            id: "Overview".into(),
            disabled: false,
        },
        CompositeItem {
            id: "Projects".into(),
            disabled: false,
        },
    ]);
    assert_eq!(controller.next(), Some(1));
    controller.set_active(Some(1));
    assert_eq!(controller.next(), Some(2));
    assert_eq!(controller.typeahead("proj"), Some(2));
    assert_eq!(controller.first(), Some(1));
    assert_eq!(controller.last(), Some(2));
}

#[test]
fn button_suite_build_and_mount() {
    let mut tree = WidgetTree::new();
    let btn = Button::new("Click me").on_click(|| ());
    let primary = PrimaryButton::new("Primary Action");
    let ghost = GhostButton::new("Ghost");
    let icon = IconButton::new("★");

    let scope = mount_ltr(&mut tree, btn);
    tree.layout(Constraints::loose(Size::new(400.0, 300.0)))
        .expect("layout");
    let root = scoped_child(&tree, scope);
    let render_id = tree.render_id(root).unwrap();
    let size = tree.render_size(render_id).unwrap();
    assert!(size.width > 0.0);
    assert!(size.height > 0.0);
    assert!(size.width < 400.0, "button should shrink-wrap its label");
    assert_eq!(size.height, 32.0);

    let _ = mount_ltr(&mut tree, primary);
    tree.layout(Constraints::loose(Size::new(400.0, 300.0)))
        .expect("layout primary");
    let _ = mount_ltr(&mut tree, ghost);
    tree.layout(Constraints::loose(Size::new(400.0, 300.0)))
        .expect("layout ghost");
    let _ = mount_ltr(&mut tree, icon);
    tree.layout(Constraints::loose(Size::new(400.0, 300.0)))
        .expect("layout icon");
}

#[test]
fn button_style_cursor_and_direct_size_constraints_reach_retained_owners() {
    let button = Button::new("Policy").style(
        ButtonStyle::new()
            .mouse_cursor("grab")
            .minimum_size(Size::new(140.0, 52.0)),
    );
    let mut tree = WidgetTree::new();
    let scope = mount_ltr(&mut tree, button);
    tree.layout(Constraints::loose(Size::new(220.0, 100.0)))
        .expect("layout styled button");
    let root = scoped_child(&tree, scope);
    let size = tree
        .render_size(tree.render_id(root).expect("render id"))
        .expect("render size");
    assert!(size.width >= 140.0, "width={}", size.width);
    assert!(size.height >= 52.0, "height={}", size.height);
    assert_eq!(
        tree.mouse_cursor_at(Offset::new(10.0, 10.0)),
        MouseCursor::Grab
    );
}

#[test]
fn input_suite_build_and_mount() {
    let mut tree = WidgetTree::new();
    let controller = TextEditingController::new();
    let text_field = TextField::new(controller.clone()).placeholder("Enter text");
    let text_area = TextArea::new(controller).placeholder("Multiline");

    let scope = mount_ltr(&mut tree, text_field);
    tree.layout(Constraints::loose(Size::new(400.0, 300.0)))
        .expect("layout");
    let root = scoped_child(&tree, scope);
    let render_id = tree.render_id(root).unwrap();
    let size = tree.render_size(render_id).unwrap();
    assert!(size.width > 0.0);

    let _ = mount_ltr(&mut tree, text_area);
    tree.layout(Constraints::loose(Size::new(400.0, 300.0)))
        .expect("layout text area");
}

#[test]
fn selection_suite_build_and_mount() {
    let mut tree = WidgetTree::new();
    let checkbox = Checkbox::new(true).label("Option A");
    let radio = Radio::new("val1", Some("val1")).label("Option 1");
    let switch = Switch::new(true);

    let scope = mount_ltr(&mut tree, checkbox);
    tree.layout(Constraints::loose(Size::new(400.0, 300.0)))
        .expect("layout");
    let root = scoped_child(&tree, scope);
    let render_id = tree.render_id(root).unwrap();
    let size = tree.render_size(render_id).unwrap();
    assert!(size.width > 0.0);

    let _ = mount_ltr(&mut tree, radio);
    tree.layout(Constraints::loose(Size::new(400.0, 300.0)))
        .expect("layout radio");
    let _ = mount_ltr(&mut tree, switch);
    tree.layout(Constraints::loose(Size::new(400.0, 300.0)))
        .expect("layout switch");
}

#[test]
fn containers_suite_build_and_mount() {
    let mut tree = WidgetTree::new();
    let card = Card::new(
        incular_widgets::Text::new("Card Content")
            .text_direction(incular_widgets::TextDirection::Ltr),
    );
    let divider = Divider::new();

    let root = tree.mount(card.into()).expect("mount card");
    tree.layout(Constraints::loose(Size::new(400.0, 300.0)))
        .expect("layout");
    let render_id = tree.render_id(root).unwrap();
    let size = tree.render_size(render_id).unwrap();
    assert!(size.width > 0.0);

    tree.mount(divider.into()).expect("mount divider");
}

#[test]
fn theme_scope_is_deferred_and_nested() {
    let light = ControlThemeScope::new(ControlTheme::light(), Card::new(Button::new("Light")));
    let dark = ControlThemeScope::new(ControlTheme::dark(), light);
    let mut tree = WidgetTree::new();
    let scope = mount_ltr(&mut tree, dark);
    tree.layout(Constraints::loose(Size::new(320.0, 120.0)))
        .expect("layout");
    let root = scoped_child(&tree, scope);
    assert!(
        tree.render_size(tree.render_id(root).unwrap())
            .unwrap()
            .width
            > 0.0
    );
}

#[test]
fn theme_scope_changes_materialized_button_surface() {
    fn surface(theme: ControlTheme) -> incular_core::Color {
        let mut tree = WidgetTree::new();
        let scope = mount_ltr(
            &mut tree,
            ControlThemeScope::new(theme, Button::new("Scoped")),
        );
        tree.layout(Constraints::loose(Size::new(240.0, 80.0)))
            .expect("layout");
        let root = scoped_child(&tree, scope);
        tree.paint()
            .commands()
            .iter()
            .find_map(|command| match command {
                PaintCommand::RRect {
                    brush: Brush::Solid(color),
                    ..
                } => Some(color.to_owned()),
                _ => None,
            })
            .unwrap_or_else(|| panic!("button {root:?} did not paint a surface"))
    }

    assert_ne!(
        surface(ControlTheme::light()),
        surface(ControlTheme::dark())
    );
}

#[test]
fn shared_state_values_resolve_specific_flags() {
    let table = StateTable::new(1_u32).hovered(2).pressed(3).disabled(4);
    assert_eq!(table.resolve(ControlState::HOVERED), 2);
    assert_eq!(
        table.resolve(ControlState::PRESSED | ControlState::HOVERED),
        3
    );
    assert_eq!(table.resolve(ControlState::DISABLED), 4);
}

#[test]
fn retained_button_interaction_keeps_hover_press_and_focus_independent() {
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(incular_widgets::internal::action(
            Size::new(96.0, 32.0),
            incular_core::Color::WHITE,
            ActionId(1),
        ))
        .expect("mount button");
    tree.layout(Constraints::loose(Size::new(160.0, 80.0)))
        .expect("layout");

    tree.set_button_interaction(root, Some(true), None, Some(true))
        .expect("set hover/focus");
    assert_eq!(tree.button_state(root), Some(ButtonState::Focused));

    tree.set_button_interaction(root, None, Some(true), None)
        .expect("set press");
    assert_eq!(tree.button_state(root), Some(ButtonState::Pressed));

    tree.set_button_interaction(root, None, Some(false), None)
        .expect("release press");
    assert_eq!(tree.button_state(root), Some(ButtonState::Focused));
}

#[test]
fn compound_parts_mount_without_visual_wrapper_requirements() {
    let root = checkbox::Root::new()
        .checked(true)
        .child(checkbox::Indicator::new());
    let tabs = tabs::Root::new().child(tabs::List::new(tabs::Tab::new(
        "one",
        incular_widgets::Text::new("One").text_direction(incular_widgets::TextDirection::Ltr),
    )));
    let mut tree = WidgetTree::new();
    tree.mount(incular_widgets::Column::new([Widget::from(root), Widget::from(tabs)]).into())
        .expect("mount compound parts");
    tree.layout(Constraints::loose(Size::new(320.0, 120.0)))
        .expect("layout");
}

#[test]
fn extended_control_inventory_has_retained_visuals() {
    let content = incular_widgets::Column::new([
        Widget::from(progress::Root::new().value(0.6).label("Loading")),
        Widget::from(meter::Root::new().value(0.4).label("Usage")),
        Widget::from(avatar::Root::new().label("Ada Lovelace")),
        Widget::from(toast::Root::new("Saved").description("Project written")),
        Widget::from(
            alert_dialog::Root::new().open(true).child(
                incular_widgets::Text::new("Confirm")
                    .text_direction(incular_widgets::TextDirection::Ltr),
            ),
        ),
        Widget::from(
            drawer::Root::new()
                .open(true)
                .child(
                    incular_widgets::Text::new("Page")
                        .text_direction(incular_widgets::TextDirection::Ltr),
                )
                .panel(
                    incular_widgets::Text::new("Drawer")
                        .text_direction(incular_widgets::TextDirection::Ltr),
                ),
        ),
        Widget::from(
            scroll_area::Root::new().child(incular_widgets::Column::new([
                incular_widgets::Text::new("Scrollable")
                    .text_direction(incular_widgets::TextDirection::Ltr),
                incular_widgets::Text::new("Content")
                    .text_direction(incular_widgets::TextDirection::Ltr),
            ])),
        ),
    ]);
    let mut tree = WidgetTree::new();
    let scope = mount_ltr(&mut tree, content);
    tree.layout(Constraints::loose(Size::new(640.0, 480.0)))
        .expect("layout");
    let root = scoped_child(&tree, scope);
    assert!(
        tree.render_size(tree.render_id(root).unwrap())
            .unwrap()
            .height
            > 0.0
    );
}
