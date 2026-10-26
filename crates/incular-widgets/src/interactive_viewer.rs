//! Flutter `InteractiveViewer` (`widgets/interactive_viewer.dart`): pans and
//! scales its child with gestures, keeps the viewport within a boundary
//! around the child, and continues pans and scales with friction.

use std::{cell::Cell, rc::Rc, time::Duration};

use incular_animation::AnimationController;
use incular_config::{Alignment, Axis, Clip, EdgeInsets};
use incular_core::{Offset, Rect, Size, Transform as CoreTransform};
use incular_gestures::constants::MIN_FLING_VELOCITY;

use crate::gestures::{ScaleEndDetails, ScaleStartDetails, ScaleUpdateDetails, Velocity};
use crate::{
    ClipRect, GestureDetector, HitTestBehavior, Listener, OverflowBox, RawPointerEvent, Transform,
    Widget,
};

/// `_kDrag`: the default `interactionEndFrictionCoefficient`.
const DRAG: f32 = 0.000_013_5;
/// `kDefaultMouseScrollToScaleFactor`: the default `scaleFactor`.
const MOUSE_SCROLL_TO_SCALE_FACTOR: f32 = 200.0;

/// Flutter `PanAxis`: which directions a pan may move the child.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PanAxis {
    /// Only horizontally.
    Horizontal,
    /// Only vertically.
    Vertical,
    /// Along whichever axis the pan started on.
    Aligned,
    /// In any direction (the default).
    #[default]
    Free,
}

/// Flutter `TransformationController`: the scene-to-viewport transform of an
/// [`InteractiveViewer`]. Reading it from a builder rebuilds that builder
/// when it changes.
#[derive(Clone, Default)]
pub struct TransformationController {
    value: Rc<Cell<CoreTransform>>,
    changes: incular_core::reactivity::DependencySource,
}

impl std::fmt::Debug for TransformationController {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TransformationController")
            .field("value", &self.value.get())
            .finish()
    }
}

impl PartialEq for TransformationController {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.value, &other.value)
    }
}

impl TransformationController {
    /// An identity transform.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_value(value: CoreTransform) -> Self {
        let controller = Self::new();
        controller.value.set(value);
        controller
    }

    #[must_use]
    pub fn value(&self) -> CoreTransform {
        self.changes.track();
        self.value.get()
    }

    pub fn set_value(&self, value: CoreTransform) {
        if self.value.get() != value {
            self.value.set(value);
            self.changes.notify();
        }
    }

    /// Flutter `toScene`: the scene point under a viewport point.
    #[must_use]
    pub fn to_scene(&self, viewport_point: Offset) -> Offset {
        self.value
            .get()
            .inverse_transform_point(viewport_point)
            .unwrap_or(viewport_point)
    }
}

/// `_GestureType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GestureType {
    Pan,
    Scale,
}

/// The parts of a rotation-free `Matrix4` the viewer manipulates: a uniform
/// scale and a translation.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Matrix {
    scale: f32,
    translation: Offset,
}

impl Matrix {
    fn of(transform: CoreTransform) -> Self {
        let [a, b, ..] = transform.to_kurbo().as_coeffs();
        Self {
            // `getMaxScaleOnAxis` of a rotation-free uniform scale.
            scale: a.hypot(b) as f32,
            translation: transform.translation_offset(),
        }
    }

    fn transform(self) -> CoreTransform {
        CoreTransform::translation(self.translation).then(CoreTransform::scale(self.scale))
    }

    /// `translateByDouble`: post-multiplies a translation.
    fn translated(self, by: Offset) -> Self {
        Self {
            translation: Offset::new(
                self.translation.x + by.x * self.scale,
                self.translation.y + by.y * self.scale,
            ),
            ..self
        }
    }

    /// `_transformViewport`: the viewport rectangle in scene coordinates.
    fn scene_viewport(self, viewport: Rect) -> Rect {
        let to_scene = |point: Offset| {
            Offset::new(
                (point.x - self.translation.x) / self.scale,
                (point.y - self.translation.y) / self.scale,
            )
        };
        let min = to_scene(viewport.origin);
        let max = to_scene(Offset::new(
            viewport.origin.x + viewport.size.width,
            viewport.origin.y + viewport.size.height,
        ));
        Rect::from_origin_size(min, Size::new(max.x - min.x, max.y - min.y))
    }
}

/// `_round`: removes floating-point residue below 1e-9.
fn round(offset: Offset) -> Offset {
    let round = |value: f32| ((f64::from(value) * 1e9).round() / 1e9) as f32;
    Offset::new(round(offset.x), round(offset.y))
}

/// `_exceedsBy` for axis-aligned rectangles: how far the viewport lies
/// outside the boundary, by its farthest corner on each axis.
fn exceeds_by(boundary: Rect, viewport: Rect) -> Offset {
    let (left, top) = (boundary.origin.x, boundary.origin.y);
    let (right, bottom) = (left + boundary.size.width, top + boundary.size.height);
    let corners = [
        viewport.origin,
        Offset::new(viewport.origin.x + viewport.size.width, viewport.origin.y),
        Offset::new(
            viewport.origin.x + viewport.size.width,
            viewport.origin.y + viewport.size.height,
        ),
        Offset::new(viewport.origin.x, viewport.origin.y + viewport.size.height),
    ];
    let mut largest = Offset::ZERO;
    for corner in corners {
        // `getNearestPointInside` of an axis-aligned rectangle.
        let inside = Offset::new(corner.x.clamp(left, right), corner.y.clamp(top, bottom));
        let excess = Offset::new(inside.x - corner.x, inside.y - corner.y);
        if excess.x.abs() > largest.x.abs() {
            largest.x = excess.x;
        }
        if excess.y.abs() > largest.y.abs() {
            largest.y = excess.y;
        }
    }
    round(largest)
}

/// `_getPanAxis`.
fn pan_axis_between(first: Offset, second: Offset) -> Option<Axis> {
    if first == second {
        return None;
    }
    let (x, y) = (second.x - first.x, second.y - first.y);
    Some(if x.abs() > y.abs() {
        Axis::Horizontal
    } else {
        Axis::Vertical
    })
}

/// `_getFinalTime`: when a friction simulation slows below
/// `effectively_motionless`.
fn final_time(velocity: f32, drag: f32, effectively_motionless: f32) -> f32 {
    (effectively_motionless / velocity).ln() / (drag / 100.0).ln()
}

/// `FrictionSimulation.finalX`.
fn friction_final_x(drag: f32, position: f32, velocity: f32) -> f32 {
    position - velocity / drag.ln()
}

/// `FrictionSimulation.x`.
fn friction_x(drag: f32, position: f32, velocity: f32, time: f32) -> f32 {
    position + velocity * (drag.powf(time) - 1.0) / drag.ln()
}

/// `Curves.decelerate`.
fn decelerate(t: f32) -> f32 {
    let inverse = 1.0 - t;
    1.0 - inverse * inverse
}

type StartCallback = Rc<dyn Fn(ScaleStartDetails)>;
type UpdateCallback = Rc<dyn Fn(ScaleUpdateDetails)>;
type EndCallback = Rc<dyn Fn(ScaleEndDetails)>;

/// A widget that pans and zooms its child.
///
/// Flutter's defaults: scale between 0.8 and 2.5, panning and scaling
/// enabled, `PanAxis::Free`, no boundary margin, a constrained child,
/// `Clip::HardEdge`, a friction coefficient of 0.0000135 and a mouse-wheel
/// scale factor of 200.
#[derive(Clone)]
pub struct InteractiveViewer {
    child: Widget,
    controller: Option<TransformationController>,
    min_scale: f32,
    max_scale: f32,
    pan_enabled: bool,
    scale_enabled: bool,
    pan_axis: PanAxis,
    boundary_margin: EdgeInsets,
    constrained: bool,
    clip_behavior: Clip,
    alignment: Option<Alignment>,
    interaction_end_friction_coefficient: f32,
    scale_factor: f32,
    on_interaction_start: Option<StartCallback>,
    on_interaction_update: Option<UpdateCallback>,
    on_interaction_end: Option<EndCallback>,
}

impl InteractiveViewer {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            child: child.into(),
            controller: None,
            min_scale: 0.8,
            max_scale: 2.5,
            pan_enabled: true,
            scale_enabled: true,
            pan_axis: PanAxis::Free,
            boundary_margin: EdgeInsets::ZERO,
            constrained: true,
            clip_behavior: Clip::HardEdge,
            alignment: None,
            interaction_end_friction_coefficient: DRAG,
            scale_factor: MOUSE_SCROLL_TO_SCALE_FACTOR,
            on_interaction_start: None,
            on_interaction_update: None,
            on_interaction_end: None,
        }
    }

    /// Shares the transform with the application.
    #[must_use]
    pub fn transformation_controller(mut self, controller: TransformationController) -> Self {
        self.controller = Some(controller);
        self
    }

    #[must_use]
    pub fn min_scale(mut self, scale: f32) -> Self {
        self.min_scale = scale;
        self
    }

    #[must_use]
    pub fn max_scale(mut self, scale: f32) -> Self {
        self.max_scale = scale;
        self
    }

    #[must_use]
    pub fn pan_enabled(mut self, enabled: bool) -> Self {
        self.pan_enabled = enabled;
        self
    }

    #[must_use]
    pub fn scale_enabled(mut self, enabled: bool) -> Self {
        self.scale_enabled = enabled;
        self
    }

    #[must_use]
    pub fn pan_axis(mut self, axis: PanAxis) -> Self {
        self.pan_axis = axis;
        self
    }

    /// Extends the boundary beyond the child; infinite margins remove it.
    #[must_use]
    pub fn boundary_margin(mut self, margin: EdgeInsets) -> Self {
        self.boundary_margin = margin;
        self
    }

    /// Whether the child gets the viewer's constraints (true) or is laid out
    /// unconstrained at the top-left.
    #[must_use]
    pub fn constrained(mut self, constrained: bool) -> Self {
        self.constrained = constrained;
        self
    }

    #[must_use]
    pub fn clip_behavior(mut self, clip: Clip) -> Self {
        self.clip_behavior = clip;
        self
    }

    /// Where within the child the transform is applied about.
    #[must_use]
    pub fn alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = Some(alignment);
        self
    }

    #[must_use]
    pub fn interaction_end_friction_coefficient(mut self, coefficient: f32) -> Self {
        self.interaction_end_friction_coefficient = coefficient;
        self
    }

    /// How much a mouse wheel scroll scales: `exp(-dy / scale_factor)`.
    #[must_use]
    pub fn scale_factor(mut self, factor: f32) -> Self {
        self.scale_factor = factor;
        self
    }

    #[must_use]
    pub fn on_interaction_start(mut self, callback: impl Fn(ScaleStartDetails) + 'static) -> Self {
        self.on_interaction_start = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn on_interaction_update(
        mut self,
        callback: impl Fn(ScaleUpdateDetails) + 'static,
    ) -> Self {
        self.on_interaction_update = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn on_interaction_end(mut self, callback: impl Fn(ScaleEndDetails) + 'static) -> Self {
        self.on_interaction_end = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn min_scale_value(&self) -> f32 {
        self.min_scale
    }

    #[must_use]
    pub fn max_scale_value(&self) -> f32 {
        self.max_scale
    }
}

/// A running pan or scale animation (`_animation`, `_scaleAnimation`).
#[derive(Clone, Copy)]
enum Inertia {
    Translation {
        from: Offset,
        to: Offset,
    },
    Scale {
        from: f32,
        to: f32,
        focal_point: Offset,
    },
}

/// `_InteractiveViewerState`.
struct ViewerState {
    config: InteractiveViewer,
    transformer: TransformationController,
    child_size: Cell<Size>,
    viewport: Cell<Size>,
    gesture_type: Cell<Option<GestureType>>,
    current_axis: Cell<Option<Axis>>,
    reference_focal_point: Cell<Option<Offset>>,
    scale_start: Cell<Option<f32>>,
    inertia_controller: AnimationController,
    inertia: Cell<Option<Inertia>>,
}

impl ViewerState {
    fn boundary_rect(&self) -> Rect {
        let size = self.child_size.get();
        let margin = self.config.boundary_margin;
        Rect::from_origin_size(
            Offset::new(-margin.left, -margin.top),
            Size::new(
                size.width + margin.left + margin.right,
                size.height + margin.top + margin.bottom,
            ),
        )
    }

    fn viewport_rect(&self) -> Rect {
        Rect::from_origin_size(Offset::ZERO, self.viewport.get())
    }

    fn matrix(&self) -> Matrix {
        Matrix::of(self.transformer.value.get())
    }

    fn set_matrix(&self, matrix: Matrix) {
        self.transformer.set_value(matrix.transform());
    }

    fn to_scene(&self, point: Offset) -> Offset {
        self.transformer.to_scene(point)
    }

    /// `_matrixTranslate`.
    fn matrix_translate(&self, matrix: Matrix, translation: Offset) -> Matrix {
        if translation == Offset::ZERO {
            return matrix;
        }
        let aligned = match (self.current_axis.get(), self.config.pan_axis) {
            (None, _) | (Some(_), PanAxis::Free) => translation,
            (Some(_), PanAxis::Horizontal) | (Some(Axis::Horizontal), PanAxis::Aligned) => {
                Offset::new(translation.x, 0.0)
            }
            (Some(_), PanAxis::Vertical) | (Some(Axis::Vertical), PanAxis::Aligned) => {
                Offset::new(0.0, translation.y)
            }
        };
        let next = matrix.translated(aligned);
        let boundary = self.boundary_rect();
        let infinite = !(boundary.origin.x.is_finite()
            && boundary.origin.y.is_finite()
            && boundary.size.width.is_finite()
            && boundary.size.height.is_finite());
        if infinite {
            return next;
        }
        let viewport = self.viewport_rect();
        let offending = exceeds_by(boundary, next.scene_viewport(viewport));
        if offending == Offset::ZERO {
            return next;
        }
        let corrected = Matrix {
            translation: Offset::new(
                next.translation.x - offending.x * matrix.scale,
                next.translation.y - offending.y * matrix.scale,
            ),
            ..matrix
        };
        let offending_corrected = exceeds_by(boundary, corrected.scene_viewport(viewport));
        if offending_corrected == Offset::ZERO {
            return corrected;
        }
        if offending_corrected.x != 0.0 && offending_corrected.y != 0.0 {
            return matrix;
        }
        Matrix {
            translation: Offset::new(
                if offending_corrected.x == 0.0 {
                    corrected.translation.x
                } else {
                    0.0
                },
                if offending_corrected.y == 0.0 {
                    corrected.translation.y
                } else {
                    0.0
                },
            ),
            ..matrix
        }
    }

    /// `_matrixScale`.
    fn matrix_scale(&self, matrix: Matrix, scale: f32) -> Matrix {
        if scale == 1.0 {
            return matrix;
        }
        let current = self.matrix().scale;
        let viewport = self.viewport.get();
        let boundary = self.boundary_rect();
        let total = (current * scale).max(
            (viewport.width / boundary.size.width).max(viewport.height / boundary.size.height),
        );
        let clamped_total = total.clamp(self.config.min_scale, self.config.max_scale);
        Matrix {
            scale: matrix.scale * (clamped_total / current),
            ..matrix
        }
    }

    fn gesture_is_supported(&self, gesture: Option<GestureType>) -> bool {
        match gesture {
            Some(GestureType::Scale) => self.config.scale_enabled,
            Some(GestureType::Pan) | None => self.config.pan_enabled,
        }
    }

    /// `_getGestureType` without rotation.
    fn gesture_type_of(&self, details: &ScaleUpdateDetails) -> GestureType {
        let scale = if self.config.scale_enabled {
            details.scale
        } else {
            1.0
        };
        if (scale - 1.0).abs() > 0.0 {
            GestureType::Scale
        } else {
            GestureType::Pan
        }
    }

    fn stop_inertia(&self) {
        if self.inertia_controller.is_active() {
            self.inertia_controller.stop();
        }
        self.inertia_controller.reset();
        self.inertia.set(None);
    }

    /// `_onScaleStart`.
    fn scale_start(&self, details: ScaleStartDetails) {
        if let Some(callback) = &self.config.on_interaction_start {
            callback(details);
        }
        self.stop_inertia();
        self.gesture_type.set(None);
        self.current_axis.set(None);
        self.scale_start.set(Some(self.matrix().scale));
        self.reference_focal_point
            .set(Some(self.to_scene(details.local_focal_point)));
    }

    /// `_onScaleUpdate` without rotation.
    fn scale_update(&self, details: ScaleUpdateDetails) {
        let scale = self.matrix().scale;
        let focal_point_scene = self.to_scene(details.local_focal_point);
        let gesture = match self.gesture_type.get() {
            Some(GestureType::Pan) | None => self.gesture_type_of(&details),
            Some(gesture) => gesture,
        };
        self.gesture_type.set(Some(gesture));
        if !self.gesture_is_supported(Some(gesture)) {
            self.notify_update(details);
            return;
        }
        let Some(reference) = self.reference_focal_point.get() else {
            return;
        };
        match gesture {
            GestureType::Scale => {
                let desired = self.scale_start.get().unwrap_or(scale) * details.scale;
                self.set_matrix(self.matrix_scale(self.matrix(), desired / scale));
                let scaled = self.to_scene(details.local_focal_point);
                self.set_matrix(self.matrix_translate(self.matrix(), scaled - reference));
                // Clamping may have kept the focal point from staying put.
                let check = self.to_scene(details.local_focal_point);
                if round(reference) != round(check) {
                    self.reference_focal_point.set(Some(check));
                }
            }
            GestureType::Pan => {
                if details.scale != 1.0 {
                    self.notify_update(details);
                    return;
                }
                if self.current_axis.get().is_none() {
                    self.current_axis
                        .set(pan_axis_between(reference, focal_point_scene));
                }
                self.set_matrix(
                    self.matrix_translate(self.matrix(), focal_point_scene - reference),
                );
                self.reference_focal_point
                    .set(Some(self.to_scene(details.local_focal_point)));
            }
        }
        self.notify_update(details);
    }

    fn notify_update(&self, details: ScaleUpdateDetails) {
        if let Some(callback) = &self.config.on_interaction_update {
            callback(details);
        }
    }

    /// `_onScaleEnd`.
    fn scale_end(&self, details: ScaleEndDetails, last_focal_point: Offset) {
        if let Some(callback) = &self.config.on_interaction_end {
            callback(details);
        }
        self.scale_start.set(None);
        self.reference_focal_point.set(None);
        self.stop_inertia();
        if !self.gesture_is_supported(self.gesture_type.get()) {
            self.current_axis.set(None);
            return;
        }
        let drag = self.config.interaction_end_friction_coefficient;
        match self.gesture_type.get() {
            Some(GestureType::Pan) => {
                let velocity = details.velocity.pixels_per_second;
                let speed = velocity.x.hypot(velocity.y);
                if speed < MIN_FLING_VELOCITY {
                    self.current_axis.set(None);
                    return;
                }
                let translation = self.matrix().translation;
                let to = Offset::new(
                    friction_final_x(drag, translation.x, velocity.x),
                    friction_final_x(drag, translation.y, velocity.y),
                );
                let duration = final_time(speed, drag, 10.0);
                self.run_inertia(
                    Inertia::Translation {
                        from: translation,
                        to,
                    },
                    duration,
                );
            }
            Some(GestureType::Scale) => {
                if details.scale_velocity.abs() < 0.1 {
                    self.current_axis.set(None);
                    return;
                }
                let scale = self.matrix().scale;
                let duration = final_time(details.scale_velocity.abs(), drag, 0.1);
                let to = friction_x(
                    drag * self.config.scale_factor,
                    scale,
                    details.scale_velocity / 10.0,
                    duration,
                );
                self.run_inertia(
                    Inertia::Scale {
                        from: scale,
                        to,
                        focal_point: last_focal_point,
                    },
                    duration,
                );
            }
            None => {}
        }
    }

    /// `_receivedPointerSignal` for a mouse wheel: scales about the pointer.
    fn pointer_signal(&self, event: RawPointerEvent) {
        if event.scroll_delta.y == 0.0 {
            return;
        }
        let scale_change = (-event.scroll_delta.y / self.config.scale_factor).exp();
        let (global, local) = (event.position, event.local_position);
        if let Some(callback) = &self.config.on_interaction_start {
            callback(ScaleStartDetails {
                focal_point: global,
                local_focal_point: local,
                pointer_count: 0,
                source_time: None,
                kind: None,
            });
        }
        let update = ScaleUpdateDetails {
            focal_point: global,
            local_focal_point: local,
            scale: scale_change,
            horizontal_scale: 1.0,
            vertical_scale: 1.0,
            rotation: 0.0,
            focal_point_delta: Offset::ZERO,
            pointer_count: 0,
            source_time: None,
        };
        if self.gesture_is_supported(Some(GestureType::Scale)) {
            let focal_point_scene = self.to_scene(local);
            self.set_matrix(self.matrix_scale(self.matrix(), scale_change));
            let scaled = self.to_scene(local);
            self.set_matrix(self.matrix_translate(self.matrix(), scaled - focal_point_scene));
        }
        self.notify_update(update);
        if let Some(callback) = &self.config.on_interaction_end {
            callback(ScaleEndDetails {
                velocity: Velocity::ZERO,
                scale_velocity: 0.0,
                pointer_count: 0,
            });
        }
    }

    fn run_inertia(&self, inertia: Inertia, seconds: f32) {
        self.inertia.set(Some(inertia));
        // `Duration(milliseconds: (tFinal * 1000).round())`.
        let millis = (seconds * 1000.0).round().max(0.0) as u64;
        self.inertia_controller
            .set_duration(Duration::from_millis(millis));
        self.inertia_controller.forward_on_next_tick();
    }

    /// `_handleInertiaAnimation` and `_handleScaleAnimation`.
    fn inertia_tick(&self, value: f32) {
        let Some(inertia) = self.inertia.get() else {
            return;
        };
        if !self.inertia_controller.is_active() {
            self.current_axis.set(None);
            self.inertia.set(None);
            return;
        }
        let t = decelerate(value);
        match inertia {
            Inertia::Translation { from, to } => {
                let target =
                    Offset::new(from.x + (to.x - from.x) * t, from.y + (to.y - from.y) * t);
                let translation = self.matrix().translation;
                self.set_matrix(self.matrix_translate(
                    self.matrix(),
                    self.to_scene(target) - self.to_scene(translation),
                ));
            }
            Inertia::Scale {
                from,
                to,
                focal_point,
            } => {
                let desired = from + (to - from) * t;
                let change = desired / self.matrix().scale;
                let reference = self.to_scene(focal_point);
                self.set_matrix(self.matrix_scale(self.matrix(), change));
                let scaled = self.to_scene(focal_point);
                self.set_matrix(self.matrix_translate(self.matrix(), scaled - reference));
            }
        }
    }
}

impl From<InteractiveViewer> for Widget {
    fn from(value: InteractiveViewer) -> Self {
        let transformer = value.controller.clone().unwrap_or_default();
        let inertia_controller = AnimationController::new(Duration::ZERO);
        let constrained = value.constrained;
        let clip_behavior = value.clip_behavior;
        let alignment = value.alignment;
        let child = value.child.clone();
        let state = Rc::new(ViewerState {
            config: value,
            transformer: transformer.clone(),
            child_size: Cell::new(Size::ZERO),
            viewport: Cell::new(Size::ZERO),
            gesture_type: Cell::new(None),
            current_axis: Cell::new(None),
            reference_focal_point: Cell::new(None),
            scale_start: Cell::new(None),
            inertia_controller: inertia_controller.clone(),
            inertia: Cell::new(None),
        });
        inertia_controller.add_listener({
            let weak = Rc::downgrade(&state);
            move |value| {
                if let Some(state) = weak.upgrade() {
                    state.inertia_tick(value);
                }
            }
        });
        // `_InteractiveViewerBuilt`.
        let built = Widget::stateful_builder(Rc::new(Cell::new(0)), {
            let state = state.clone();
            move |_| {
                let matrix = state.transformer.value();
                let observed = {
                    let state = state.clone();
                    Widget::size_observer(move |size| state.child_size.set(size), child.clone())
                };
                let transformed: Widget = match alignment {
                    None => Transform::new(matrix, observed).into(),
                    Some(alignment) => {
                        let size = state.child_size.get();
                        Transform::new(matrix, observed)
                            .origin(Offset::new(
                                size.width * (alignment.x + 1.0) / 2.0,
                                size.height * (alignment.y + 1.0) / 2.0,
                            ))
                            .into()
                    }
                };
                let content: Widget = if constrained {
                    transformed
                } else {
                    OverflowBox::builder()
                        .min_width(0.0)
                        .min_height(0.0)
                        .max_width(f32::INFINITY)
                        .max_height(f32::INFINITY)
                        .child(transformed)
                        .build()
                        .into()
                };
                if clip_behavior == Clip::None {
                    content
                } else {
                    ClipRect::new(content).clip_behavior(clip_behavior).into()
                }
            }
        });
        let last_focal_point = Rc::new(Cell::new(Offset::ZERO));
        let detector = GestureDetector::new(Widget::animation_ticker(inertia_controller, built))
            .behavior(HitTestBehavior::Opaque)
            .on_scale_start({
                let state = state.clone();
                move |details| state.scale_start(details)
            })
            .on_scale_update({
                let state = state.clone();
                let last_focal_point = last_focal_point.clone();
                move |details| {
                    last_focal_point.set(details.local_focal_point);
                    state.scale_update(details);
                }
            })
            .on_scale_end({
                let state = state.clone();
                move |details| state.scale_end(details, last_focal_point.get())
            });
        let signal_state = state.clone();
        Listener::new(Widget::size_observer(
            move |size| state.viewport.set(size),
            Widget::from(detector),
        ))
        .on_pointer_signal(move |event| signal_state.pointer_signal(event))
        .into()
    }
}
