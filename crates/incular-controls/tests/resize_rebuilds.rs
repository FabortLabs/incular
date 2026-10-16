//! Controls read their theme at build time, not layout time: resizing a
//! column of controls must not rebuild any of them, while a theme change
//! still must.

use incular_config::Constraints;
use incular_controls::prelude::*;
use incular_core::Size;
use incular_widgets::{
    Column, Directionality, TextDirection, Widget,
    internal::{TextEditingController, WidgetTree},
};

fn controls() -> Widget {
    let mut children: Vec<Widget> = Vec::new();
    for index in 0..10 {
        children.push(Button::new(format!("Button {index}")).into());
        children.push(PrimaryButton::new("Primary").into());
        children.push(Checkbox::new(index % 2 == 0).label("Checkbox").into());
        children.push(Radio::new("a", Some("a")).label("Radio").into());
        children.push(Switch::new(true).into());
        children.push(
            TextField::new(TextEditingController::new())
                .placeholder("Text")
                .into(),
        );
        children.push(Card::new(Button::new("In card")).into());
        children.push(Divider::new().into());
        children.push(Progress::new().value(0.5).into());
        children.push(Meter::new().value(0.25).into());
        children.push(Avatar::new().label("AB").into());
    }
    Column::new(children).into()
}

fn themed(theme: ControlTheme, child: Widget) -> Widget {
    Directionality::new(TextDirection::Ltr, ControlThemeScope::new(theme, child)).into()
}

#[test]
fn resizing_controls_rebuilds_none_but_theme_change_rebuilds_them() {
    let column = controls();
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(themed(ControlTheme::light(), column.clone()))
        .expect("mount control column");
    let height = 20_000.0;
    tree.layout(Constraints::tight(Size::new(400.0, height)))
        .expect("initial layout");

    let mut resize_rebuilds = 0;
    for step in 1..=10 {
        let before = tree.diagnostics();
        let width = 400.0 + step as f32 * 7.0;
        tree.layout(Constraints::tight(Size::new(width, height)))
            .expect("resized layout");
        let after = tree.diagnostics();
        assert!(
            after.layouts > before.layouts,
            "a resize must lay the controls out again"
        );
        assert_eq!(after.mounts, before.mounts, "a resize must not remount");
        resize_rebuilds += after.rebuilds - before.rebuilds;
    }
    eprintln!("rebuilds over 10 resizes of 110 controls: {resize_rebuilds}");
    assert_eq!(resize_rebuilds, 0, "a resize must not rebuild any control");

    let before = tree.diagnostics();
    tree.update(root, themed(ControlTheme::dark(), column))
        .expect("switch theme");
    tree.layout(Constraints::tight(Size::new(470.0, height)))
        .expect("layout after theme change");
    let theme_rebuilds = tree.diagnostics().rebuilds - before.rebuilds;
    eprintln!("rebuilds after a theme change: {theme_rebuilds}");
    assert!(
        theme_rebuilds >= 110,
        "a theme change must rebuild every control, got {theme_rebuilds} rebuilds"
    );
}
