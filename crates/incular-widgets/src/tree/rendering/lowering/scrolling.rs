use super::super::super::*;

pub(super) fn lower_scrolling(widget: &Widget, context: &DependencyContext) -> RenderKind {
    match widget.kind() {
        WidgetKind::Scroll {
            controller,
            axis,
            reverse,
            physics,
            inherit_physics,
            keyboard_dismiss,
            clip_behavior,
            drag_start_behavior,
            ..
        } => {
            let behavior = ambient_scroll_behavior(context);
            // Flutter resolves the ambient horizontal direction first,
            // then applies the caller's reverse flag.
            let rtl = *axis == Axis::Horizontal
                && context.depend::<TextDirection>() == Some(TextDirection::Rtl);
            let mut resolved = behavior.resolve();
            if let Some(dismiss) = keyboard_dismiss {
                resolved.keyboard_dismiss = *dismiss;
            }
            RenderKind::Scroll {
                controller: controller.clone(),
                axis: *axis,
                reverse: *reverse ^ rtl,
                physics: inherit_physics
                    .map_or(*physics, |page| inherited_physics(&behavior, page)),
                clip_behavior: *clip_behavior,
                drag_start_behavior: *drag_start_behavior,
                behavior: resolved,
            }
        }
        WidgetKind::RawScrollbar {
            controller, style, ..
        } => RenderKind::RawScrollbar {
            controller: controller.clone(),
            style: *style,
            text_direction: if style.orientation == crate::RawScrollbarOrientation::Auto {
                context.depend::<TextDirection>().unwrap_or(TextDirection::Ltr)
            } else {
                TextDirection::Ltr
            },
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
            let physics = config.inherit_physics.map_or(config.physics, |page| {
                let physics = inherited_physics(&behavior, page);
                config
                    .physics
                    .page_geometry_fraction()
                    .map_or(physics, |fraction| {
                        physics
                            .with_page_geometry(fraction)
                            .with_implicit_scrolling(config.physics.allows_implicit_scrolling())
                    })
            });
            use crate::scrolling::SliverKeyboardDismissal;
            let mut resolved = behavior.resolve();
            resolved.keyboard_dismiss = match config.keyboard_dismissal {
                SliverKeyboardDismissal::Disabled => crate::ScrollViewKeyboardDismissBehavior::Manual,
                SliverKeyboardDismissal::Inherited => resolved.keyboard_dismiss,
                SliverKeyboardDismissal::Explicit(policy) => policy,
            };
            let text_direction = context.depend::<TextDirection>();
            let safe_insets = context
                .depend::<crate::tree::context::RuntimeSafeInsets>()
                .map_or(EdgeInsets::ZERO, |padding| padding.0);
            let config = if physics == config.physics
                && resolved == config.behavior
                && text_direction == config.text_direction
                && safe_insets == config.safe_insets
            {
                config.clone()
            } else {
                let mut config = (**config).clone();
                config.physics = physics;
                config.behavior = resolved;
                config.text_direction = text_direction;
                config.safe_insets = safe_insets;
                if text_direction == Some(TextDirection::Rtl) {
                    use crate::scrolling::SliverAxisResolution;
                    match &config.axis_resolution {
                        SliverAxisResolution::Physical => {}
                        SliverAxisResolution::Inherited => config.reverse = !config.reverse,
                        SliverAxisResolution::InheritedWithDelegate(alternate) => {
                            config.reverse = !config.reverse;
                            config.delegate = alternate.get();
                        }
                    }
                }
                Rc::new(config)
            };
            RenderKind::SliverViewport { config }
        }
        _ => unreachable!("scroll lowering received a non-scrolling widget"),
    }
}

/// The nearest `ScrollConfiguration`'s behavior, else Flutter's default
/// `ScrollBehavior`.
fn ambient_scroll_behavior(context: &DependencyContext) -> Rc<crate::ScrollBehavior> {
    context
        .depend_shared::<crate::ScrollBehavior>()
        .unwrap_or_default()
}

/// Physics for a view that set none: the behavior's, with page snapping on
/// top for a `PageView` (Flutter's `_kPagePhysics.applyTo`).
fn inherited_physics(behavior: &crate::ScrollBehavior, page: bool) -> ScrollPhysics {
    let physics = behavior.get_scroll_physics();
    if page { physics.page() } else { physics }
}
