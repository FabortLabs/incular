//! Flattening culls whole subtrees by memoized picture bounds. Every change
//! that moves a picture must reach the memo, so each step here compares the
//! flattened pictures with the ones whose world bounds meet the clip.

use incular_core::{Color, Offset, Rect, Size, Transform};
use incular_rendering::*;

const ROW: f32 = 10.0;
const ROWS: usize = 50;

fn viewport() -> Rect {
    Rect::from_origin_size(Offset::ZERO, Size::new(100.0, 40.0))
}

fn picture(tree: &mut LayerTree, size: Size) -> LayerId {
    let mut list = DisplayList::new();
    list.push(PaintCommand::Rect {
        rect: Rect::from_origin_size(Offset::ZERO, size),
        color: Color::WHITE,
    });
    tree.create_picture(list, Rect::from_origin_size(Offset::ZERO, size))
}

/// A scroll viewport: clip > content transform > one row per child, each a
/// transform placing a nested transform and picture.
struct List {
    tree: LayerTree,
    content: LayerId,
    rows: Vec<LayerId>,
    pictures: Vec<LayerId>,
}

impl List {
    fn new() -> Self {
        let mut tree = LayerTree::new();
        let clip = tree.create_clip_rect(viewport());
        let content = tree.create_transform(Transform::IDENTITY);
        let mut rows = Vec::new();
        let mut pictures = Vec::new();
        for index in 0..ROWS {
            let row =
                tree.create_transform(Transform::translation(Offset::new(0.0, index as f32 * ROW)));
            let inset = tree.create_transform(Transform::translation(Offset::new(2.0, 1.0)));
            let picture = picture(&mut tree, Size::new(50.0, ROW - 2.0));
            tree.set_children(inset, vec![picture]);
            tree.set_children(row, vec![inset]);
            rows.push(row);
            pictures.push(picture);
        }
        tree.set_children(content, rows.clone());
        tree.set_children(clip, vec![content]);
        tree.set_root(clip);
        Self {
            tree,
            content,
            rows,
            pictures,
        }
    }

    /// Flattens and returns the drawn pictures, checking each one's world
    /// bounds meet the viewport.
    fn drawn(&mut self) -> Vec<LayerId> {
        let _ = self.tree.flatten();
        let drawn = self.tree.flattened_pictures().to_vec();
        for picture in &drawn {
            assert!(viewport().intersects(picture.world_bounds), "{picture:?}");
        }
        drawn.iter().map(|picture| picture.layer).collect()
    }

    fn rows_drawn(&mut self, rows: std::ops::Range<usize>) {
        let expected = self.pictures[rows].to_vec();
        assert_eq!(self.drawn(), expected);
    }
}

#[test]
fn off_screen_rows_are_culled_whole() {
    let mut list = List::new();
    let before = list.tree.diagnostics().layers_culled;
    list.rows_drawn(0..4);
    assert_eq!(
        list.tree.diagnostics().layers_culled - before,
        (ROWS - 4) as u64,
        "each off-screen row is culled at its own root"
    );
}

#[test]
fn scrolling_the_content_transform_moves_the_cull() {
    let mut list = List::new();
    list.rows_drawn(0..4);
    list.tree.update_transform(
        list.content,
        Transform::translation(Offset::new(0.0, -205.0)),
    );
    list.rows_drawn(20..25);
    list.tree.update_transform(
        list.content,
        Transform::translation(Offset::new(0.0, -10.0)),
    );
    list.rows_drawn(1..5);
}

#[test]
fn moving_a_nested_transform_or_picture_updates_the_cull() {
    let mut list = List::new();
    list.rows_drawn(0..4);

    // Move row 30 up into the viewport through its own transform.
    list.tree.update_transform(
        list.rows[30],
        Transform::translation(Offset::new(0.0, 15.0)),
    );
    let drawn = list.drawn();
    assert!(drawn.contains(&list.pictures[30]));

    // Grow row 40's picture until it reaches the viewport from below.
    let mut tall_list = DisplayList::new();
    tall_list.push(PaintCommand::Rect {
        rect: Rect::from_origin_size(Offset::new(0.0, -390.0), Size::new(50.0, 400.0)),
        color: Color::WHITE,
    });
    list.tree.update_picture(
        list.pictures[40],
        tall_list,
        Rect::from_origin_size(Offset::new(0.0, -390.0), Size::new(50.0, 400.0)),
    );
    assert!(list.drawn().contains(&list.pictures[40]));
}

#[test]
fn reparenting_and_removal_update_the_cull() {
    let mut list = List::new();
    list.rows_drawn(0..4);

    // Move row 45's content under row 0, which is on screen.
    let inset = list
        .tree
        .create_transform(Transform::translation(Offset::new(2.0, 1.0)));
    list.tree.set_children(inset, vec![list.pictures[45]]);
    list.tree.set_children(list.rows[45], Vec::new());
    list.tree.set_children(list.rows[0], vec![inset]);
    let drawn = list.drawn();
    assert!(drawn.contains(&list.pictures[45]));
    assert!(!drawn.contains(&list.pictures[0]));

    // Removing the adopted picture leaves the row empty and culled.
    list.tree.remove(list.pictures[45]);
    assert!(!list.drawn().contains(&list.pictures[45]));
}

#[test]
fn off_screen_surface_partitions_still_reach_the_output() {
    let mut list = List::new();
    let partition = SurfacePartitionId::from_parts(3, 1);
    list.tree
        .set_surface_partition(list.rows[30], Some(partition));
    let output = list.tree.flatten();
    assert!(
        output
            .commands()
            .iter()
            .any(|command| *command == PaintCommand::PushSurfacePartition { id: partition })
    );
    list.tree.clear_surface_partitions();
    let output = list.tree.flatten();
    assert!(
        !output
            .commands()
            .iter()
            .any(|command| matches!(command, PaintCommand::PushSurfacePartition { .. }))
    );
}
