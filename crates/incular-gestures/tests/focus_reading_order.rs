//! Flutter `ReadingOrderTraversalPolicy` band grouping and the
//! `DirectionalFocusTraversalPolicyMixin` candidate search.

use incular_core::{Offset, Rect, Size};
use incular_gestures::{
    DirectionalFocusCandidate, TraversalDirection, directional_focus_index, reading_order_by,
};

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_origin_size(Offset::new(x, y), Size::new(width, height))
}

fn order(items: &[(&'static str, Rect)]) -> Vec<&'static str> {
    reading_order_by(items.to_vec(), |item| item.1)
        .into_iter()
        .map(|item| item.0)
        .collect()
}

#[test]
fn band_overlap_groups_staggered_items_left_to_right() {
    // `b` starts 15 px lower than `a` but still overlaps its band, so it comes
    // first because it is further left. Flutter's `_pickNext` reads the band.
    let items = [
        ("a", rect(60., 0., 20., 20.)),
        ("b", rect(0., 15., 20., 20.)),
        ("c", rect(0., 60., 20., 20.)),
    ];
    assert_eq!(order(&items), ["b", "a", "c"]);
}

#[test]
fn items_outside_the_band_keep_top_to_bottom_order() {
    // `b` starts below `a`'s bottom edge, so it is in a later band even though
    // it is further left.
    let items = [
        ("a", rect(40., 0., 20., 20.)),
        ("b", rect(0., 25., 20., 20.)),
    ];
    assert_eq!(order(&items), ["a", "b"]);
}

#[test]
fn equal_rectangles_keep_registration_order() {
    let items = [("x", rect(0., 0., 20., 20.)), ("y", rect(0., 0., 20., 20.))];
    assert_eq!(order(&items), ["x", "y"]);
}

#[test]
fn disjoint_rows_use_stable_horizontal_order_even_at_touching_edges() {
    let items = [
        ("last", rect(0., 30., 10., 20.)),
        ("right", rect(50., 0., 10., 20.)),
        ("first", rect(10., 0., 10., 30.)),
        ("second", rect(10., 0., 10., 10.)),
    ];
    assert_eq!(order(&items), ["first", "second", "right", "last"]);
}

#[test]
fn a_tall_earlier_member_can_reach_into_the_next_row() {
    let items = [
        ("tall", rect(50., 0., 10., 50.)),
        ("short", rect(40., 0., 10., 10.)),
        ("lower", rect(0., 30., 10., 10.)),
    ];
    assert_eq!(order(&items), ["lower", "short", "tall"]);
}

#[test]
fn zero_width_item_is_excluded_from_its_row_band() {
    let items = [
        ("right", rect(50., 0., 10., 10.)),
        ("empty", rect(0., 0., 0., 10.)),
        ("middle", rect(25., 0., 10., 10.)),
    ];
    assert_eq!(order(&items), ["middle", "right", "empty"]);
}

fn candidates(rects: &[Rect]) -> Vec<DirectionalFocusCandidate> {
    rects
        .iter()
        .map(|rect| DirectionalFocusCandidate {
            rect: *rect,
            shares_scroll_owner: true,
        })
        .collect()
}

#[test]
fn arrow_moves_to_the_adjacent_item_in_a_row() {
    // Flutter `DirectionalFocusIntent`: a row of three 40 px controls.
    let row = [
        rect(0., 0., 40., 30.),
        rect(40., 0., 40., 30.),
        rect(80., 0., 40., 30.),
    ];
    let cands = candidates(&row);
    assert_eq!(
        directional_focus_index(Some(row[0]), &cands, TraversalDirection::Right),
        Some(1)
    );
    assert_eq!(
        directional_focus_index(Some(row[1]), &cands, TraversalDirection::Left),
        Some(0)
    );
}

#[test]
fn directional_search_wraps_to_the_far_side_at_an_edge() {
    // With nothing further right, Flutter's closed-loop edge behaviour wraps to
    // the far side of the same band: right of the last item is the first one.
    let row = [
        rect(0., 0., 40., 30.),
        rect(40., 0., 40., 30.),
        rect(80., 0., 40., 30.),
    ];
    let cands = candidates(&row);
    assert_eq!(
        directional_focus_index(Some(row[2]), &cands, TraversalDirection::Right),
        Some(0)
    );
    // Left of the first item wraps to the last one, the same way.
    assert_eq!(
        directional_focus_index(Some(row[0]), &cands, TraversalDirection::Left),
        Some(2)
    );
}

#[test]
fn vertical_arrows_follow_a_column() {
    let column = [
        rect(0., 0., 40., 30.),
        rect(0., 40., 40., 30.),
        rect(0., 80., 40., 30.),
    ];
    let cands = candidates(&column);
    assert_eq!(
        directional_focus_index(Some(column[0]), &cands, TraversalDirection::Down),
        Some(1)
    );
    assert_eq!(
        directional_focus_index(Some(column[0]), &cands, TraversalDirection::Up),
        Some(2)
    );
}

#[test]
fn out_of_band_targets_pick_the_closest_edge() {
    // `c` shares the column of `a`'s right half and is closer along x than
    // `b`, which lies in a different column.
    let a = rect(0., 0., 40., 30.);
    let b = rect(100., 40., 40., 30.);
    let c = rect(40., 40., 40., 30.);
    let cands = candidates(&[a, b, c]);
    assert_eq!(
        directional_focus_index(Some(a), &cands, TraversalDirection::Down),
        Some(2)
    );
}

#[test]
fn candidates_sharing_the_scroll_owner_are_preferred() {
    let a = rect(0., 0., 40., 30.);
    let outside = DirectionalFocusCandidate {
        rect: rect(0., 40., 40., 30.),
        shares_scroll_owner: false,
    };
    let inside = DirectionalFocusCandidate {
        rect: rect(0., 80., 40., 30.),
        shares_scroll_owner: true,
    };
    assert_eq!(
        directional_focus_index(Some(a), &[outside, inside], TraversalDirection::Down),
        Some(1)
    );
}

#[test]
fn without_a_focused_item_the_edge_item_in_the_direction_is_chosen() {
    let row = [
        rect(0., 0., 40., 30.),
        rect(40., 0., 40., 30.),
        rect(80., 0., 40., 30.),
    ];
    let cands = candidates(&row);
    assert_eq!(
        directional_focus_index(None, &cands, TraversalDirection::Right),
        Some(0)
    );
    assert_eq!(
        directional_focus_index(None, &cands, TraversalDirection::Left),
        Some(2)
    );
}
