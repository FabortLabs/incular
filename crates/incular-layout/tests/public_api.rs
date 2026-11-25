use incular_layout::{
    Align, Alignment, AlignmentChild, AlignmentDirectional, Constraints, CrossAxisAlignment,
    EdgeInsets, Flex, FlexChild, MainAxisAlignment, MainAxisSize, Offset, Size, Table, TableChild,
    TextDirection, Wrap, WrapChild, layout_align, layout_flex, layout_padding, layout_table,
    layout_wrap,
};

#[test]
fn alignment_places_child_in_remaining_space() {
    assert_eq!(
        Alignment::CENTER.within(Size::new(100.0, 80.0), Size::new(20.0, 10.0)),
        Offset::new(40.0, 35.0)
    );
    assert_eq!(
        AlignmentDirectional::CENTER_START.resolve(TextDirection::Rtl),
        Alignment::CENTER_RIGHT
    );
}

#[test]
fn flex_distributes_tight_children_and_preserves_order() {
    let config = Flex::row()
        .with_main_axis_size(MainAxisSize::Max)
        .with_main_axis_alignment(MainAxisAlignment::Start)
        .with_cross_axis_alignment(CrossAxisAlignment::Start);
    let result = layout_flex(
        Constraints::tight(Size::new(100.0, 20.0)),
        &[
            FlexChild::new(Size::new(10.0, 5.0)),
            FlexChild::expanded(Size::new(1.0, 5.0), 1),
        ],
        config,
    );
    assert_eq!(result.size, Size::new(100.0, 20.0));
    assert_eq!(result.children[0].offset, Offset::ZERO);
    assert_eq!(result.children[1].offset.x, 10.0);
    assert_eq!(result.children[1].size.width, 90.0);
}

#[test]
fn padding_and_alignment_have_expected_geometry() {
    let padded = layout_padding(
        Constraints::unbounded(),
        Size::new(10.0, 8.0),
        EdgeInsets::all(2.0),
    );
    assert_eq!(padded.size, Size::new(14.0, 12.0));
    let aligned = layout_align(
        Constraints::tight(Size::new(100.0, 50.0)),
        AlignmentChild::new(Size::new(20.0, 10.0)),
        Align::new(Alignment::CENTER),
    );
    assert_eq!(aligned.children[0].offset, Offset::new(40.0, 20.0));
}

#[test]
fn wrap_breaks_lines_at_the_main_axis_bound() {
    let result = layout_wrap(
        Constraints::new(25.0, 25.0, 0.0, f32::INFINITY),
        &[
            WrapChild::new(Size::new(10.0, 4.0)),
            WrapChild::new(Size::new(10.0, 5.0)),
            WrapChild::new(Size::new(10.0, 6.0)),
        ],
        Wrap::default(),
    );
    assert_eq!(result.children[0].offset, Offset::ZERO);
    assert_eq!(result.children[2].offset.y, 5.0);
}

#[test]
fn table_uses_max_content_columns() {
    let result = layout_table(
        Constraints::unbounded(),
        &[
            TableChild::new(Size::new(20.0, 4.0)),
            TableChild::new(Size::new(10.0, 5.0)),
            TableChild::new(Size::new(5.0, 3.0)),
        ],
        Table::new(2),
    );
    assert_eq!(result.size, Size::new(30.0, 8.0));
    assert_eq!(result.children[2].offset, Offset::new(0.0, 5.0));
}

#[test]
fn wrap_ranges_match_explicit_index_runs_in_both_reading_directions() {
    let tags: Vec<_> = (0..257)
        .map(|i| WrapChild::new(Size::new(((17 * i) % 91) as f32, 12.)))
        .collect();
    for width in [1., 90., 256., 1024.] {
        let mut rows = Vec::new();
        let mut row = Vec::new();
        let mut used = 0.;
        for (index, tag) in tags.iter().enumerate() {
            if !row.is_empty() && used + tag.size.width > width {
                rows.push(std::mem::take(&mut row));
                used = 0.;
            }
            row.push(index);
            used += tag.size.width;
        }
        rows.push(row);
        for rtl in [false, true] {
            let config = Wrap {
                text_direction: if rtl {
                    incular_layout::TextDirection::Rtl
                } else {
                    incular_layout::TextDirection::Ltr
                },
                ..Wrap::default()
            };
            let layout = layout_wrap(
                Constraints::new(width, width, 0., f32::INFINITY),
                &tags,
                config,
            );
            for (line, row) in rows.iter().enumerate() {
                let row_width: f32 = row.iter().map(|&i| tags[i].size.width).sum();
                let mut x = if rtl { width.max(row_width) } else { 0. };
                for &i in row {
                    if rtl {
                        x -= tags[i].size.width;
                    }
                    assert_eq!(layout.children[i].offset, Offset::new(x, line as f32 * 12.));
                    if !rtl {
                        x += tags[i].size.width;
                    }
                }
            }
        }
    }
}

#[test]
fn rtl_table_offsets_are_relative_to_the_natural_grid_in_a_wider_box() {
    use incular_layout::TableColumnWidth;
    let cells = [
        TableChild::new(Size::new(100., 10.)),
        TableChild::new(Size::new(110., 10.)),
        TableChild::new(Size::new(125., 10.)),
    ];
    let mut config = Table::new(3);
    config.column_widths = vec![
        (0, TableColumnWidth::Fixed(100.)),
        (1, TableColumnWidth::Fixed(110.)),
        (2, TableColumnWidth::Fixed(125.)),
    ];
    config.column_spacing = 7.;
    let constraints = Constraints::tight(Size::new(400., 20.));
    let offsets = |config: &Table| {
        let layout = layout_table(constraints, &cells, config);
        assert_eq!(layout.size.width, 400.);
        layout
            .children
            .into_iter()
            .map(|child| child.offset.x)
            .collect::<Vec<_>>()
    };
    assert_eq!(offsets(&config), [0., 107., 224.]);
    config.text_direction = TextDirection::Rtl;
    assert_eq!(offsets(&config), [249., 132., 0.]);
}
