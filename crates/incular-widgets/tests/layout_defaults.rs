//! Flutter-parity defaults for layout descriptors.

use incular_config::{
    Alignment, Axis, Clip, CrossAxisAlignment, MainAxisAlignment, MainAxisSize, StackFit,
    VerticalDirection, WrapAlignment, WrapCrossAlignment,
};
use incular_widgets::{Column, Flex, IndexedStack, Row, Stack, Table, Widget, Wrap};

fn assert_flex_defaults(
    children: &[Widget],
    main_axis_alignment: MainAxisAlignment,
    main_axis_size: MainAxisSize,
    cross_axis_alignment: CrossAxisAlignment,
    text_direction: Option<incular_config::TextDirection>,
    vertical_direction: VerticalDirection,
    spacing: f32,
) {
    assert!(children.is_empty());
    assert_eq!(main_axis_alignment, MainAxisAlignment::Start);
    assert_eq!(main_axis_size, MainAxisSize::Max);
    assert_eq!(cross_axis_alignment, CrossAxisAlignment::Center);
    assert_eq!(text_direction, None);
    assert_eq!(vertical_direction, VerticalDirection::Down);
    assert_eq!(spacing, 0.0);
}

#[test]
fn flex_defaults_match_flutter() {
    let row = Row::default();
    assert_flex_defaults(
        &row.children,
        row.main_axis_alignment,
        row.main_axis_size,
        row.cross_axis_alignment,
        row.text_direction,
        row.vertical_direction,
        row.spacing,
    );

    let column = Column::default();
    assert_flex_defaults(
        &column.children,
        column.main_axis_alignment,
        column.main_axis_size,
        column.cross_axis_alignment,
        column.text_direction,
        column.vertical_direction,
        column.spacing,
    );

    let flex = Flex::default();
    assert_eq!(flex.direction, Axis::Horizontal);
    assert_flex_defaults(
        &flex.children,
        flex.main_axis_alignment,
        flex.main_axis_size,
        flex.cross_axis_alignment,
        flex.text_direction,
        flex.vertical_direction,
        flex.spacing,
    );
}

#[test]
fn stack_defaults_match_flutter() {
    assert_eq!(
        Stack::default(),
        Stack::builder()
            .children(Vec::<Widget>::new())
            .alignment(Alignment::TOP_LEFT)
            .fit(StackFit::Loose)
            .clip_behavior(Clip::HardEdge)
            .build()
    );

    assert_eq!(
        IndexedStack::default(),
        IndexedStack::builder()
            .index(0)
            .alignment(Alignment::TOP_LEFT)
            .fit(StackFit::Loose)
            .clip_behavior(Clip::HardEdge)
            .children(Vec::<Widget>::new())
            .build()
    );
}

#[test]
fn wrap_defaults_match_flutter() {
    assert_eq!(
        Wrap::default(),
        Wrap::builder()
            .children(Vec::<Widget>::new())
            .direction(Axis::Horizontal)
            .alignment(WrapAlignment::Start)
            .spacing(0.0)
            .run_alignment(WrapAlignment::Start)
            .run_spacing(0.0)
            .cross_axis_alignment(WrapCrossAlignment::Start)
            .vertical_direction(VerticalDirection::Down)
            .build()
    );
}

#[test]
fn table_defaults_match_flutter() {
    assert_eq!(
        Table::default(),
        Table::builder()
            .columns(1)
            .column_spacing(0.0)
            .row_spacing(0.0)
            .children(Vec::<Widget>::new())
            .build()
    );
}
