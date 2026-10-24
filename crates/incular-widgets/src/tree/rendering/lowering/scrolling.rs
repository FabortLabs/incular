use super::super::super::*;

pub(super) fn lower_scrolling(widget: &Widget, context: &DependencyContext) -> RenderKind {
    match widget.kind() {
        WidgetKind::Scroll {
            controller,
            axis,
            reverse,
            physics,
            inherit_physics,
            clip_behavior,
            drag_start_behavior,
            ..
        } => {
            let behavior = ambient_scroll_behavior(context);
            RenderKind::Scroll {
                controller: controller.clone(),
                axis: *axis,
                reverse: *reverse,
                physics: inherit_physics
                    .map_or(*physics, |page| inherited_physics(&behavior, page)),
                clip_behavior: *clip_behavior,
                drag_start_behavior: *drag_start_behavior,
                behavior: behavior.resolve(),
            }
        }
        WidgetKind::RawScrollbar {
            controller, style, ..
        } => RenderKind::RawScrollbar {
            controller: controller.clone(),
            style: *style,
        },
        WidgetKind::ListWheelScrollView { config } => RenderKind::ListWheelScrollView {
            config: config.clone(),
        },
        WidgetKind::ListWheelViewport { config } => RenderKind::ListWheelViewport {
            config: config.clone(),
        },
        WidgetKind::DraggableScrollableSheet { config } => RenderKind::DraggableScrollableSheet {
            config: config.clone(),
        },
        WidgetKind::DraggableScrollableActuator { actuator, .. } => {
            RenderKind::DraggableScrollableActuator {
                actuator: actuator.clone(),
            }
        }
        WidgetKind::TwoDimensionalScrollView { config } => RenderKind::TwoDimensionalScrollView {
            config: config.clone(),
        },
        WidgetKind::TwoDimensionalViewport { config } => RenderKind::TwoDimensionalViewport {
            config: config.clone(),
        },
        WidgetKind::PersistentHeader {
            controller,
            axis,
            reverse,
            pinned,
            ..
        } => RenderKind::PersistentHeader {
            controller: controller.clone(),
            axis: *axis,
            reverse: *reverse,
            pinned: *pinned,
        },
        WidgetKind::NotificationListener { .. } => RenderKind::Gesture,
        WidgetKind::SliverViewport { config } => {
            let behavior = ambient_scroll_behavior(context);
            let physics = config
                .inherit_physics
                .map_or(config.physics, |page| inherited_physics(&behavior, page));
            let resolved = behavior.resolve();
            let config = if physics == config.physics && resolved == config.behavior {
                config.clone()
            } else {
                let mut config = (**config).clone();
                config.physics = physics;
                config.behavior = resolved;
                Rc::new(config)
            };
            RenderKind::SliverViewport { config }
        }
        _ => unreachable!("scroll lowering received a non-scrolling widget"),
    }
}

/// The nearest `ScrollConfiguration`'s behavior, else Flutter's default
/// `ScrollBehavior`.
fn ambient_scroll_behavior(context: &DependencyContext) -> crate::ScrollBehavior {
    context
        .depend::<crate::ScrollBehavior>()
        .unwrap_or_default()
}

/// Physics for a view that set none: the behavior's, with page snapping on
/// top for a `PageView` (Flutter's `_kPagePhysics.applyTo`).
fn inherited_physics(behavior: &crate::ScrollBehavior, page: bool) -> ScrollPhysics {
    let physics = behavior.get_scroll_physics();
    if page { physics.page() } else { physics }
}
