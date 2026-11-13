//! Cost of flattening a retained layer tree after one change: a scrolled
//! list behind a clip, and a screen of cards under nested effect layers.
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use incular_core::{Color, Offset, Rect, Size, Transform};
use incular_rendering::{DisplayList, DropShadowEffect, LayerId, LayerTree, PaintCommand, RRect};

fn row_rect() -> Rect {
    Rect::from_origin_size(Offset::ZERO, Size::new(400., 40.))
}

fn picture(shade: u8) -> DisplayList {
    let mut list = DisplayList::new();
    list.push(PaintCommand::Rect {
        rect: row_rect(),
        color: Color::rgba(shade, shade, shade, 255),
    });
    list
}

/// A viewport clip over one scrolled transform holding `rows` placed rows.
fn scrolled_list(rows: usize) -> (LayerTree, LayerId) {
    let mut tree = LayerTree::new();
    let children = (0..rows)
        .map(|index| {
            let picture = tree.create_picture(picture(index as u8), row_rect());
            let placed =
                tree.create_transform(Transform::translation(Offset::new(0., index as f32 * 40.)));
            tree.set_children(placed, vec![picture]);
            placed
        })
        .collect();
    let scroll = tree.create_transform(Transform::IDENTITY);
    tree.set_children(scroll, children);
    let viewport =
        tree.create_clip_rect(Rect::from_origin_size(Offset::ZERO, Size::new(400., 600.)));
    tree.set_children(viewport, vec![scroll]);
    tree.set_root(viewport);
    (tree, scroll)
}

/// A fading page (`depth` nested opacity groups, as stacked route and
/// switcher transitions produce) over `cards` cards, each a rounded clip in
/// a drop shadow in an opacity group. Returns the first card's picture.
fn effect_cards(cards: usize, depth: usize) -> (LayerTree, LayerId) {
    let mut tree = LayerTree::new();
    let mut first = None;
    let children = (0..cards)
        .map(|index| {
            let picture = tree.create_picture(picture(index as u8), row_rect());
            first.get_or_insert(picture);
            let clip = tree.create_clip_rrect(RRect::new(
                row_rect(),
                incular_rendering::CornerRadii::uniform(8.),
            ));
            tree.set_children(clip, vec![picture]);
            let shadow = tree.create_drop_shadow(DropShadowEffect::new(
                Offset::new(0., 2.),
                4.,
                Color::rgba(0, 0, 0, 80),
            ));
            tree.set_children(shadow, vec![clip]);
            let fade = tree.create_opacity(0.9);
            tree.set_children(fade, vec![shadow]);
            let placed = tree.create_transform(Transform::translation(Offset::new(
                (index % 4) as f32 * 410.,
                (index / 4) as f32 * 50.,
            )));
            tree.set_children(placed, vec![fade]);
            placed
        })
        .collect();
    let mut root = tree.create_transform(Transform::IDENTITY);
    tree.set_children(root, children);
    for _ in 0..depth {
        let fade = tree.create_opacity(0.5);
        tree.set_children(fade, vec![root]);
        root = fade;
    }
    tree.set_root(root);
    (tree, first.expect("at least one card"))
}

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("flatten");
    for rows in [200_usize, 5_000] {
        let (mut tree, scroll) = scrolled_list(rows);
        let mut offset = 0.;
        group.bench_with_input(BenchmarkId::new("scroll", rows), &rows, |b, _| {
            b.iter(|| {
                offset = (offset + 7.) % 2_000.;
                tree.update_transform(scroll, Transform::translation(Offset::new(0., -offset)));
                std::hint::black_box(tree.flatten())
            });
        });
    }
    for (cards, depth) in [(200_usize, 1_usize), (200, 3), (1_000, 3)] {
        let (mut tree, card) = effect_cards(cards, depth);
        let mut shade = 0_u8;
        group.bench_with_input(
            BenchmarkId::new("effect_cards", format!("{cards}x{depth}")),
            &cards,
            |b, _| {
                b.iter(|| {
                    // One card repaints; everything else is unchanged.
                    shade = shade.wrapping_add(1);
                    tree.update_picture(card, picture(shade), row_rect());
                    std::hint::black_box(tree.flatten())
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
