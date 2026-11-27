use std::sync::Arc;

use incular_core::{Color, Offset, Rect, Size, Transform};
use incular_rendering::{
    BlendMode, ClipCoverage, DisplayList, FillRule, GaussianBlur, LayerId, LayerTree, PaintCommand,
    Path, RRect,
};

fn unit() -> Rect {
    Rect::from_origin_size(Offset::ZERO, Size::new(1., 1.))
}

struct Scene {
    tree: LayerTree,
    layer: LayerId,
    picture: LayerId,
}

impl Scene {
    fn new(stage: impl FnOnce(&mut LayerTree) -> LayerId) -> Self {
        let mut tree = LayerTree::new();
        let parent = tree.create_transform(Transform::IDENTITY);
        let layer = stage(&mut tree);
        let mut display = DisplayList::new();
        display.push(PaintCommand::Rect {
            rect: unit(),
            color: Color::WHITE,
        });
        let picture = tree.create_picture(display, unit());
        tree.set_children(layer, vec![picture]);
        // The sibling keeps a published buffer even if the tested clip
        // removes every draw in its branch.
        let mut background = DisplayList::new();
        background.push(PaintCommand::Rect {
            rect: unit(),
            color: Color::BLACK,
        });
        let background = tree.create_picture(background, unit());
        tree.set_children(parent, vec![background, layer]);
        tree.set_root(parent);
        Self {
            tree,
            layer,
            picture,
        }
    }

    fn picture_bounds(&self) -> Rect {
        self.tree
            .flattened_pictures()
            .iter()
            .find(|placement| placement.layer == self.picture)
            .expect("picture is placed")
            .world_bounds
    }

    fn mutate(&mut self, change: impl FnOnce(&mut LayerTree, LayerId) -> bool) -> DisplayList {
        let before = self.tree.flatten();
        assert!(
            self.tree.flatten().shares_buffer(&before),
            "the scene begins clean"
        );
        assert!(
            change(&mut self.tree, self.layer),
            "the change invalidates the retained scene"
        );
        let after = self.tree.flatten();
        assert!(!after.shares_buffer(&before));
        assert_ne!(
            after, before,
            "the new stage reaches the flattened command stream"
        );
        assert!(
            self.tree.flatten().shares_buffer(&after),
            "publishing makes the scene clean again"
        );
        assert!(
            self.tree.layer(self.picture).is_some(),
            "the underlying picture is retained"
        );
        after
    }
}

#[test]
fn changing_offset_reissues_the_scene_with_the_new_picture_position() {
    let mut scene = Scene::new(|tree| tree.create_transform(Transform::IDENTITY));
    let _ = scene.tree.flatten();
    assert_eq!(scene.picture_bounds(), unit());
    let transform = Transform::translation(Offset::new(1., 1.));
    scene.mutate(|tree, stage| tree.update_transform(stage, transform));
    assert_eq!(
        scene.picture_bounds(),
        Rect::from_origin_size(Offset::new(1., 1.), Size::new(1., 1.))
    );
}

#[test]
fn rectangular_clip_geometry_and_coverage_each_reissue_the_scene() {
    let mut scene = Scene::new(|tree| tree.create_clip_rect(Rect::default()));
    let changed = scene.mutate(|tree, stage| tree.update_clip(stage, unit()));
    assert!(changed.commands().iter().any(|op| matches!(op,
        PaintCommand::PushClip { rect } if *rect == unit()
    )));
    let isolated = scene.mutate(|tree, stage| {
        tree.update_clip_rect_coverage(stage, ClipCoverage::AntiAliasWithSaveLayer)
    });
    assert!(isolated.commands().iter().any(|op| matches!(op,
        PaintCommand::PushClipRRect { rrect, coverage: ClipCoverage::AntiAliasWithSaveLayer, isolation: Some(pass) }
            if *rrect == RRect::uniform(unit(), 0.) && pass.layer == scene.layer
    )));
}

#[test]
fn rounded_clip_geometry_and_coverage_each_reissue_the_scene() {
    let mut scene = Scene::new(|tree| tree.create_clip_rrect(RRect::uniform(Rect::default(), 0.)));
    let rounded = RRect::uniform(unit(), 0.);
    let changed = scene.mutate(|tree, stage| tree.update_clip_rrect(stage, rounded));
    assert!(changed.commands().iter().any(|op| matches!(op,
        PaintCommand::PushClipRRect { rrect, coverage: ClipCoverage::HardEdge, isolation: None }
            if *rrect == rounded
    )));
    let isolated = scene.mutate(|tree, stage| {
        tree.update_clip_rrect_coverage(stage, ClipCoverage::AntiAliasWithSaveLayer)
    });
    assert!(isolated.commands().iter().any(|op| matches!(op,
        PaintCommand::PushClipRRect { rrect, coverage: ClipCoverage::AntiAliasWithSaveLayer, isolation: Some(pass) }
            if *rrect == rounded && pass.layer == scene.layer
    )));
}

#[test]
fn path_clip_geometry_and_coverage_each_reissue_the_scene() {
    let mut scene = Scene::new(|tree| {
        tree.create_clip_path(Arc::new(Path::builder().build()), FillRule::NonZero)
    });
    let mut builder = Path::builder();
    builder
        .move_to(Offset::ZERO)
        .line_to(Offset::new(1., 0.))
        .line_to(Offset::new(1., 1.))
        .line_to(Offset::new(0., 1.))
        .close();
    let rect = Arc::new(builder.build());
    let changed =
        scene.mutate(|tree, stage| tree.update_clip_path(stage, rect.clone(), FillRule::NonZero));
    assert!(changed.commands().iter().any(|op| matches!(op,
        PaintCommand::PushClipPath { path, fill_rule: FillRule::NonZero, coverage: ClipCoverage::HardEdge }
            if path.bounds() == Some(unit()) && Arc::ptr_eq(path, &rect)
    )));
    let isolated = scene.mutate(|tree, stage| {
        tree.update_clip_path_coverage(stage, ClipCoverage::AntiAliasWithSaveLayer)
    });
    assert!(isolated.commands().iter().any(|op| matches!(op,
        PaintCommand::PushClipPath { path, fill_rule: FillRule::NonZero, coverage: ClipCoverage::AntiAliasWithSaveLayer }
            if Arc::ptr_eq(path, &rect)
    )));
}

#[test]
fn backdrop_filter_sigma_change_reissues_the_scene() {
    let mut scene = Scene::new(|tree| {
        tree.create_backdrop_filter(GaussianBlur::new(0., 0.), BlendMode::SrcOver, true)
    });
    let blur = GaussianBlur::new(1., 0.);
    let changed = scene
        .mutate(|tree, stage| tree.update_backdrop_filter(stage, blur, BlendMode::SrcOver, true));
    assert!(changed.commands().iter().any(|op| matches!(op,
        PaintCommand::PushBackdropFilter { layer, blur: sigma, blend_mode: BlendMode::SrcOver, enabled: true, .. }
            if *layer == scene.layer && *sigma == blur
    )));
}

#[test]
fn changing_opacity_alpha_or_offset_reissues_the_same_picture() {
    let mut scene = Scene::new(|tree| tree.create_opacity(0.));
    let placement = scene.tree.create_transform(Transform::IDENTITY);
    scene.tree.set_children(scene.layer, vec![placement]);
    scene.tree.set_children(placement, vec![scene.picture]);
    let alpha = 1. / 255.;
    let changed = scene.mutate(|tree, stage| tree.update_opacity(stage, alpha));
    assert!(changed.commands().iter().any(|op| matches!(op,
        PaintCommand::PushOpacity { layer, alpha: emitted, bounds, .. }
            if *layer == scene.layer && *emitted == alpha && *bounds == unit()
    )));
    let translation = Transform::translation(Offset::new(1., 1.));
    let changed = scene.mutate(|tree, _| tree.update_transform(placement, translation));
    let moved = Rect::from_origin_size(Offset::new(1., 1.), Size::new(1., 1.));
    assert_eq!(scene.picture_bounds(), moved);
    assert!(changed.commands().iter().any(|op| matches!(op,
        PaintCommand::PushOpacity { layer, alpha: emitted, bounds, .. }
            if *layer == scene.layer && *emitted == alpha && *bounds == moved
    )));
}

#[test]
fn changing_a_color_filter_reissues_its_mode_and_color() {
    use incular_rendering::ColorFilter;

    let red = ColorFilter::blend(Color::rgba(255, 0, 0, 255), BlendMode::Color);
    let green = ColorFilter::blend(Color::rgba(0, 255, 0, 255), BlendMode::Color);
    let mut scene = Scene::new(|tree| tree.create_color_filter(red));
    let changed = scene.mutate(|tree, stage| tree.update_color_filter(stage, green));
    assert!(changed.commands().iter().any(|op| matches!(op,
        PaintCommand::PushColorFilter { layer, filter, bounds, .. }
            if *layer == scene.layer && *filter == green && *bounds == unit()
    )));
}

#[test]
fn shader_mask_bounds_blend_and_shader_each_reissue_the_scene() {
    use incular_rendering::{Brush, GradientStop, GradientStops, RadialGradient};

    let stops = GradientStops::new(vec![
        GradientStop {
            offset: 0.,
            color: Color::rgba(0, 0, 0, 0),
        },
        GradientStop {
            offset: 1.,
            color: Color::rgba(0, 0, 1, 0),
        },
    ]);
    let old_shader = Brush::RadialGradient(RadialGradient::new(Offset::ZERO, 0., stops.clone()));
    let shader = Brush::RadialGradient(RadialGradient::new(Offset::new(0.5, 0.5), 0.5, stops));
    let mut scene = Scene::new(|tree| {
        tree.create_shader_mask(
            old_shader.clone(),
            BlendMode::Clear,
            Size::ZERO,
            Transform::IDENTITY,
        )
    });
    let changed = scene.mutate(|tree, stage| {
        tree.update_shader_mask(
            stage,
            old_shader.clone(),
            BlendMode::Clear,
            unit().size,
            Transform::IDENTITY,
        )
    });
    assert!(changed.commands().iter().any(|op| matches!(op,
        PaintCommand::PushShaderMask { layer, shader, blend_mode: BlendMode::Clear, mask_size, mask_transform, .. }
            if *layer == scene.layer && shader == &old_shader && *mask_size == unit().size
                && **mask_transform == Transform::IDENTITY
    )));
    let changed =
        scene.mutate(|tree, stage| tree.update_shader_mask_blend(stage, BlendMode::Color));
    assert!(changed.commands().iter().any(|op| matches!(op,
        PaintCommand::PushShaderMask { shader, blend_mode: BlendMode::Color, mask_size, .. }
            if shader == &old_shader && *mask_size == unit().size
    )));
    let changed = scene.mutate(|tree, stage| {
        tree.update_shader_mask(
            stage,
            shader.clone(),
            BlendMode::Color,
            unit().size,
            Transform::IDENTITY,
        )
    });
    assert!(changed.commands().iter().any(|op| matches!(op,
        PaintCommand::PushShaderMask { shader: emitted, blend_mode: BlendMode::Color, mask_size, .. }
            if emitted == &shader && *mask_size == unit().size
    )));
}

#[test]
fn superellipse_clip_shape_and_coverage_each_reissue_the_scene() {
    use incular_rendering::CornerRadii;
    let primitive = |rect, radii| {
        let mut path = Path::builder();
        path.rounded_superellipse(rect, radii);
        Arc::new(path.build())
    };
    let mut scene = Scene::new(|tree| {
        tree.create_clip_path(
            primitive(Rect::default(), CornerRadii::ZERO),
            FillRule::NonZero,
        )
    });
    let unit_shape = primitive(unit(), CornerRadii::ZERO);
    let changed = scene
        .mutate(|tree, stage| tree.update_clip_path(stage, unit_shape.clone(), FillRule::NonZero));
    assert!(changed.commands().iter().any(|op| matches!(op,
        PaintCommand::PushClipPath { path, fill_rule: FillRule::NonZero, coverage: ClipCoverage::HardEdge }
            if path.bounds() == Some(unit()) && path.id() == unit_shape.id()
    )));
    let changed = scene.mutate(|tree, stage| {
        tree.update_clip_path_coverage(stage, ClipCoverage::AntiAliasWithSaveLayer)
    });
    assert!(changed.commands().iter().any(|op| matches!(op,
        PaintCommand::PushClipPath { path, fill_rule: FillRule::NonZero, coverage: ClipCoverage::AntiAliasWithSaveLayer }
            if path.id() == unit_shape.id()
    )));
    let rounded = primitive(unit(), CornerRadii::uniform(0.25));
    let changed = scene
        .mutate(|tree, stage| tree.update_clip_path(stage, rounded.clone(), FillRule::NonZero));
    assert!(changed.commands().iter().any(|op| matches!(op,
        PaintCommand::PushClipPath { path, fill_rule: FillRule::NonZero, coverage: ClipCoverage::AntiAliasWithSaveLayer }
            if path.id() == rounded.id() && !path.contains(Offset::ZERO, FillRule::NonZero)
    )));
}
