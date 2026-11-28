//! Built-in descriptor payloads and the sealed widget taxonomy.
//!
//! These types describe immutable widget configuration only. They do not own mounted identity, layout state, or rendering resources.

use super::*;
use crate::util::same_rc;

type AnimationRetargetCarry = dyn Fn(&dyn Any);

#[doc(hidden)]
#[derive(Clone)]
pub struct AnimationRetargetBridge {
    payload: Rc<dyn Any>,
    type_id: TypeId,
    carry: Rc<AnimationRetargetCarry>,
}

impl AnimationRetargetBridge {
    #[must_use]
    pub fn new<T: Any>(payload: Rc<T>, carry: impl Fn(&T) + 'static) -> Self {
        Self {
            payload,
            type_id: TypeId::of::<T>(),
            carry: Rc::new(move |previous| {
                if let Some(previous) = previous.downcast_ref::<T>() {
                    carry(previous);
                }
            }),
        }
    }

    pub(crate) fn carry_from(&self, previous: &Self) {
        if self.type_id == previous.type_id {
            (self.carry)(previous.payload.as_ref());
        }
    }
}

/// Source-side image effects, separate from the destination blend.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RasterSourceEffects {
    pub(crate) color: Option<Color>,
    pub(crate) color_blend_mode: Option<BlendMode>,
    pub(crate) invert_colors: bool,
    pub(crate) opacity: f32,
}

impl Default for RasterSourceEffects {
    fn default() -> Self {
        Self {
            color: None,
            color_blend_mode: None,
            invert_colors: false,
            opacity: 1.0,
        }
    }
}

/// Retained per-frame work ticked on the tree's animation clock (time
/// scale and `TickerMode` muting included) before the build and again in
/// the compositor pass.
pub trait FrameTick {
    /// Advances to `now`; returns whether anything changed.
    fn tick(&self, now: Instant) -> bool;
    /// Whether the delegate still has work and needs frames. This is a
    /// repeatable, non-consuming query: the tree may poll before a tick,
    /// while muted, and again after reporting geometry. Consume pending
    /// work only from [`Self::tick`].
    fn is_active(&self) -> bool;
    /// Whether [`Self::geometry`] should be called after each layout.
    fn wants_geometry(&self) -> bool {
        false
    }
    /// The ticker node's laid-out geometry, reported in the compositor pass.
    fn geometry(&self, _geometry: FrameGeometry) {}
    /// The element owning this ticker is being removed: start an exit and
    /// call `done` once it settles. Returns `false` to be removed at once.
    fn begin_exit(&self, _done: Rc<dyn Fn()>) -> bool {
        false
    }
    /// The departing element was re-inserted: return to rest.
    fn cancel_exit(&self) {}
    /// Name and value pairs DevTools shows on the ticker node.
    fn debug_properties(&self) -> Vec<(&'static str, String)> {
        Vec::new()
    }
}

/// Where a ticker node sits after layout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameGeometry {
    /// Layout rect relative to the nearest scrollable's content (or the
    /// root), ignoring every compositor transform: stable while scrolling.
    pub content_rect: Rect,
    /// Layout rect on screen: layout offsets plus scroll translations,
    /// ignoring compositor transforms (motion, `Transform`).
    pub layout_rect: Rect,
    /// The painted bounds on screen, through every transform.
    pub visual_rect: Rect,
    /// The visible region: the root intersected with every enclosing
    /// scroll viewport, on screen.
    pub viewport: Rect,
    /// Identity of the nearest enclosing scrollable (0 for the root).
    pub scroll_scope: u64,
    /// The nearest ancestor layout rect on screen that differs from the
    /// node's own (pass-through wrappers are skipped; own rect at the root).
    pub parent_rect: Rect,
}

#[doc(hidden)]
#[derive(Clone)]
pub struct FrameDelegate {
    tick: Rc<dyn FrameTick>,
    /// The idle controller the ticker node carries; never started.
    pub(crate) idle: AnimationController,
    pub(crate) revision: Rc<Cell<u64>>,
}

impl FrameDelegate {
    #[must_use]
    pub fn new(tick: Rc<dyn FrameTick>) -> Self {
        Self {
            tick,
            idle: AnimationController::new(std::time::Duration::ZERO),
            revision: Rc::new(Cell::new(0)),
        }
    }
    pub(crate) fn tick(&self, now: Instant) -> bool {
        self.tick.tick(now)
    }
    pub(crate) fn is_active(&self) -> bool {
        self.tick.is_active()
    }
    pub(crate) fn wants_geometry(&self) -> bool {
        self.tick.wants_geometry()
    }
    pub(crate) fn geometry(&self, geometry: FrameGeometry) {
        self.tick.geometry(geometry);
    }
    pub(crate) fn begin_exit(&self, done: Rc<dyn Fn()>) -> bool {
        self.tick.begin_exit(done)
    }
    pub(crate) fn cancel_exit(&self) {
        self.tick.cancel_exit();
    }
    #[cfg_attr(not(feature = "devtools"), allow(dead_code))]
    pub(crate) fn debug_properties(&self) -> Vec<(&'static str, String)> {
        self.tick.debug_properties()
    }
}

impl std::fmt::Debug for FrameDelegate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FrameDelegate").finish_non_exhaustive()
    }
}

impl PartialEq for FrameDelegate {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.tick, &other.tick)
    }
}

impl std::fmt::Debug for AnimationRetargetBridge {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AnimationRetargetBridge")
            .finish_non_exhaustive()
    }
}

impl PartialEq for AnimationRetargetBridge {
    fn eq(&self, other: &Self) -> bool {
        self.type_id == other.type_id && Rc::ptr_eq(&self.payload, &other.payload)
    }
}

/// Application-authored semantic metadata for a visual widget that does not
/// have a more specific built-in semantic role. Incular keeps this data in its
/// retained `SemanticsTree`; native adapters only project that tree.
#[derive(Clone, Debug, PartialEq)]
pub struct ExplicitSemantics {
    pub role: SemanticRole,
    pub label: Option<String>,
    pub value: Option<String>,
    pub description: Option<String>,
    pub state: SemanticState,
    pub actions: Vec<SemanticActionKind>,
}

impl ExplicitSemantics {
    #[must_use]
    pub fn new(role: SemanticRole) -> Self {
        Self {
            role,
            label: None,
            value: None,
            description: None,
            state: SemanticState::default(),
            actions: Vec::new(),
        }
    }

    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    #[must_use]
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    #[must_use]
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    #[must_use]
    pub fn state(mut self, state: SemanticState) -> Self {
        self.state = state;
        self
    }

    #[must_use]
    pub fn actions(mut self, actions: impl IntoIterator<Item = SemanticActionKind>) -> Self {
        self.actions = actions.into_iter().collect();
        self
    }
}

/// A render-phase source for the node's own accessibility rectangle.
#[derive(Clone)]
pub(super) struct SemanticBoundsInsets(pub(super) Rc<incular_core::ValueNotifier<EdgeInsets>>);

impl std::fmt::Debug for SemanticBoundsInsets {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SemanticBoundsInsets").finish_non_exhaustive()
    }
}

impl PartialEq for SemanticBoundsInsets {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Clone)]
pub(super) struct OwnedFocusCreation(pub(super) Rc<dyn Fn(incular_gestures::FocusNode)>);

impl std::fmt::Debug for OwnedFocusCreation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("OwnedFocusCreation(..)")
    }
}

impl PartialEq for OwnedFocusCreation {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct SemanticProperties {
    pub(super) label: Option<String>,
    /// Pronunciation for an implicit text label, not its explicit replacement.
    pub(super) implicit_text_attributes: Option<std::sync::Arc<incular_semantics::SemanticTextAttributes>>,
    pub(super) paragraph_parts: Option<std::sync::Arc<[super::semantics::paragraph::ParagraphSemanticPart]>>,
    pub(super) description: Option<String>,
    pub(super) hidden: bool,
    /// Boxed: it is several hundred bytes and most widgets that carry
    /// semantic metadata set only a label or a flag.
    pub(super) explicit: Option<Box<ExplicitSemantics>>,
    pub(super) bounds_insets: Option<SemanticBoundsInsets>,
    pub(super) merge_descendants: bool,
    pub(super) tag_for_children: Option<incular_semantics::SemanticsTag>,
    pub(super) locale_for_subtree: Option<std::sync::Arc<str>>,
    pub(crate) traversal_parent: Option<std::sync::Arc<str>>,
    pub(crate) traversal_child: Option<std::sync::Arc<str>>,
    /// Flutter `alwaysIncludeSemantics` on an opacity widget: keep the child's
    /// semantics even while the opacity is zero.
    pub(super) always_include_semantics: bool,
    pub(super) block_previous_siblings: bool,
    /// Flutter's `blockUserActions`: affects semantic action exposure only.
    pub(super) block_user_actions: bool,
    pub(super) focus_traversal_policy: Option<FocusTraversalPolicyKind>,
    pub(super) owns_focus_group: bool,
    pub(super) custom_focus_policy: Option<crate::CustomFocusTraversalPolicy>,
    pub(super) custom_focus_request: Option<crate::focus_keyboard::CustomFocusRequest>,
    pub(super) owned_focus_creation: Option<OwnedFocusCreation>,
    pub(super) focus_traversal_order: Option<incular_gestures::FocusOrder>,
    pub(super) exclude_focus: bool,
    pub(super) exclude_focus_traversal: bool,
    pub(super) undo_history_max_entries: Option<usize>,
    pub(super) autofill_group: Option<incular_platform::autofill::AutofillContextAction>,
    pub(super) focus_scope_autofocus: bool,
    pub(super) navigation_scope_autofocus: bool,
    pub(super) text_input_type: Option<TextInputTypeHint>,
    pub(super) text_input_action: Option<TextInputActionHint>,
    pub(super) callbacks: SemanticCallbacks,
    /// Parent-data request from a `KeepAlive` wrapper: the sliver that
    /// materializes this child keeps it retained outside the cache window.
    pub(super) keep_alive: bool,
    pub(super) focus_keep_alive: bool,
    pub(super) automatic_keep_alive: bool,
    pub(super) automatic_keep_alive_scope: bool,
    pub(super) implicit_keep_alive_scope: bool,
    /// Keep this sliver child in accessibility outside its viewport cache.
    pub(super) ensures_sliver_semantics: bool,
    /// A rendered sliver child's current geometry keeps it outside the
    /// viewport's scrolling pane. Unlike paint pinning this can change
    /// before or after the first pixel of overlap.
    pub(super) sliver_scroll_excluded: bool,
    pub(super) preserves_semantic_child_order: bool,
    // Private sliver parent data, transferred to the retained render
    // before comparing the declarative child. Never part of its identity.
    pub(super) sliver_layer_placements: Option<Rc<[crate::scrolling::sliver_layer::SliverLayerPlacement]>>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct GestureSemanticPolicy {
    allowed: u8,
    pub(crate) included: bool,
    pub(crate) scroll_factor: f32,
}

impl Default for GestureSemanticPolicy {
    fn default() -> Self {
        Self {
            allowed: 0b11_1111,
            included: true,
            scroll_factor: 0.8,
        }
    }
}

impl GestureSemanticPolicy {
    pub(crate) const ACTIONS: [SemanticActionKind; 6] = [
        SemanticActionKind::Activate,
        SemanticActionKind::LongPress,
        SemanticActionKind::ScrollLeft,
        SemanticActionKind::ScrollRight,
        SemanticActionKind::ScrollUp,
        SemanticActionKind::ScrollDown,
    ];

    const fn flag(action: SemanticActionKind) -> u8 {
        match action {
            SemanticActionKind::Activate => 1 << 0,
            SemanticActionKind::LongPress => 1 << 1,
            SemanticActionKind::ScrollLeft => 1 << 2,
            SemanticActionKind::ScrollRight => 1 << 3,
            SemanticActionKind::ScrollUp => 1 << 4,
            SemanticActionKind::ScrollDown => 1 << 5,
            _ => 0,
        }
    }

    pub(crate) fn set_allowed(&mut self, actions: impl IntoIterator<Item = SemanticActionKind>) {
        self.allowed = actions
            .into_iter()
            .fold(0, |mask, action| mask | Self::flag(action));
    }

    pub(super) fn allows(self, action: SemanticActionKind) -> bool {
        self.included && self.allowed & Self::flag(action) != 0
    }
}

type SemanticActionCallback = Rc<dyn Fn() + 'static>;

#[derive(Clone, Default)]
pub(super) struct SemanticCallbacks {
    pub(super) gesture_policy: Option<GestureSemanticPolicy>,
    pub(super) custom: Vec<crate::semantics::CustomActionBinding>,
    pub(super) bound: Vec<crate::semantics::BoundSemanticAction>,
    pub(super) activate: Option<Rc<dyn Fn() + 'static>>,
    pub(super) increment: Option<Rc<dyn Fn() + 'static>>,
    pub(super) decrement: Option<Rc<dyn Fn() + 'static>>,
    pub(super) scroll_forward: Option<Rc<dyn Fn() + 'static>>,
    pub(super) scroll_backward: Option<Rc<dyn Fn() + 'static>>,
    directional_scroll: Option<Box<[Option<SemanticActionCallback>; 4]>>,
    pub(super) long_press: Option<Rc<dyn Fn() + 'static>>,
    pub(super) dismiss: Option<Rc<dyn Fn() + 'static>>,
}

impl std::fmt::Debug for SemanticCallbacks {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SemanticCallbacks")
            .field("gesture_policy", &self.gesture_policy)
            .field("custom", &self.custom)
            .field("bound", &self.bound)
            .field("activate", &self.activate.is_some())
            .field("increment", &self.increment.is_some())
            .field("decrement", &self.decrement.is_some())
            .field("scroll_forward", &self.scroll_forward.is_some())
            .field("scroll_backward", &self.scroll_backward.is_some())
            .field(
                "directional_scroll",
                &self.directional_scroll.as_ref().map(|callbacks| {
                    std::array::from_fn::<_, 4, _>(|index| callbacks[index].is_some())
                }),
            )
            .field("long_press", &self.long_press.is_some())
            .field("dismiss", &self.dismiss.is_some())
            .finish()
    }
}

impl PartialEq for SemanticCallbacks {
    fn eq(&self, other: &Self) -> bool {
        self.gesture_policy == other.gesture_policy
            && self.custom == other.custom
            && self.bound == other.bound
            && same_rc(&self.activate, &other.activate)
            && same_rc(&self.increment, &other.increment)
            && same_rc(&self.decrement, &other.decrement)
            && same_rc(&self.scroll_forward, &other.scroll_forward)
            && same_rc(&self.scroll_backward, &other.scroll_backward)
            && match (&self.directional_scroll, &other.directional_scroll) {
                (Some(a), Some(b)) => a.iter().zip(b.iter()).all(|(a, b)| same_rc(a, b)),
                (None, None) => true,
                _ => false,
            }
            && same_rc(&self.long_press, &other.long_press)
            && same_rc(&self.dismiss, &other.dismiss)
    }
}

impl SemanticCallbacks {
    fn direction_index(action: SemanticActionKind) -> Option<usize> {
        match action {
            SemanticActionKind::ScrollLeft => Some(0),
            SemanticActionKind::ScrollRight => Some(1),
            SemanticActionKind::ScrollUp => Some(2),
            SemanticActionKind::ScrollDown => Some(3),
            _ => None,
        }
    }

    pub(super) fn set_directional_scroll(
        &mut self,
        action: SemanticActionKind,
        callback: SemanticActionCallback,
    ) {
        if let Some(index) = Self::direction_index(action) {
            self.directional_scroll
                .get_or_insert_with(|| Box::new([None, None, None, None]))[index] = Some(callback);
        }
    }

    pub(super) fn callback(&self, action: SemanticActionKind) -> Option<Rc<dyn Fn() + 'static>> {
        if let Some(index) = Self::direction_index(action) {
            return self
                .directional_scroll
                .as_ref()
                .and_then(|callbacks| callbacks[index].clone());
        }
        match action {
            SemanticActionKind::Activate => self.activate.clone(),
            SemanticActionKind::Increment => self.increment.clone(),
            SemanticActionKind::Decrement => self.decrement.clone(),
            SemanticActionKind::ScrollForward => self.scroll_forward.clone(),
            SemanticActionKind::ScrollBackward => self.scroll_backward.clone(),
            SemanticActionKind::LongPress => self.long_press.clone(),
            SemanticActionKind::Dismiss => self.dismiss.clone(),
            SemanticActionKind::Focus
            | SemanticActionKind::SetText
            | SemanticActionKind::SetSelection
            | SemanticActionKind::Copy
            | SemanticActionKind::Cut
            | SemanticActionKind::Paste
            | SemanticActionKind::Expand
            | SemanticActionKind::Collapse
            | SemanticActionKind::MoveCursorForwardByCharacter
            | SemanticActionKind::MoveCursorBackwardByCharacter
            | SemanticActionKind::MoveCursorForwardByWord
            | SemanticActionKind::MoveCursorBackwardByWord
            | SemanticActionKind::DidGainAccessibilityFocus
            | SemanticActionKind::DidLoseAccessibilityFocus
            | SemanticActionKind::ScrollToOffset
            | SemanticActionKind::ShowOnScreen
            | SemanticActionKind::ScrollLeft
            | SemanticActionKind::ScrollRight
            | SemanticActionKind::ScrollUp
            | SemanticActionKind::ScrollDown
            | SemanticActionKind::Custom => None,
        }
    }

    pub(super) fn bound_callback(
        &self,
        action: SemanticActionKind,
    ) -> Option<crate::semantics::BoundSemanticCallback> {
        self.bound
            .iter()
            .find(|binding| binding.kind == action)
            .map(|binding| binding.callback.clone())
    }

    pub(super) fn supports(&self, action: SemanticActionKind) -> bool {
        self.callback(action).is_some() || self.bound.iter().any(|binding| binding.kind == action)
    }
}
#[doc(hidden)]
#[derive(Clone)]
pub struct ButtonSpec {
    pub(crate) size: Size,
    pub(crate) color: Color,
    pub(crate) hover_color: Option<Color>,
    pub(crate) pressed_color: Option<Color>,
    pub(crate) focused_color: Option<Color>,
    pub(crate) disabled_color: Option<Color>,
    pub(crate) enabled: bool,
    pub(crate) focusable_when_disabled: bool,
    pub(crate) action: ActionId,
    pub(crate) callback: Option<Rc<dyn Fn()>>,
    pub(crate) hover_action: ActionId,
    pub(crate) hover_callback: Option<Rc<dyn Fn()>>,
    pub(crate) exit_action: ActionId,
    pub(crate) exit_callback: Option<Rc<dyn Fn()>>,
    pub(crate) has_callback: bool,
    pub(crate) child: Option<Widget>,
    pub(crate) interaction: Option<crate::internal::ActionInteractionController>,
    pub(crate) policy: crate::internal::ActionPolicy,
    pub(crate) mouse_cursor: crate::MouseCursor,
}

pub(crate) type EditTransform = Rc<
    dyn Fn(
        &incular_text::TextEditingValue,
        &incular_text::TextEditingValue,
    ) -> incular_text::TextEditingValue,
>;
pub(crate) type EditChanged = Rc<dyn Fn(&str)>;
pub(crate) type EditSelectionChanged =
    Rc<dyn Fn(incular_text::TextSelection, incular_text::SelectionChangedCause)>;
pub(crate) type EditableOutsideTap = Rc<dyn Fn(incular_gestures::RawPointerEvent)>;

pub(crate) type AppPrivateCommandCallback =
    Rc<dyn Fn(&str, &serde_json::Map<String, serde_json::Value>)>;

#[doc(hidden)]
#[derive(Clone)]
pub struct TextFieldSpec {
    pub(crate) controller: TextEditingController,
    pub(crate) group_id: crate::TapRegionGroupId,
    pub(crate) focus_node: Option<crate::FocusNode>,
    pub(crate) autofocus: bool,
    pub(crate) select_all_on_focus: Option<bool>,
    pub(crate) edit_transform: Option<EditTransform>,
    pub(crate) edit_changed: Option<EditChanged>,
    pub(crate) edit_selection_changed: Option<EditSelectionChanged>,
    pub(crate) size: Size,
    pub(crate) style: TextStyle,
    pub(crate) text_scaler: Option<TextScaler>,
    pub(crate) strut: Option<Box<StrutStyle>>,
    pub(crate) placeholder: String,
    pub(crate) placeholder_color: Color,
    pub(crate) focused_border: Option<(incular_rendering::Border, f32)>,
    pub(crate) focused_border_outset: incular_config::EdgeInsets,
    pub(crate) content_padding: incular_config::EdgeInsets,
    pub(crate) clip_behavior: incular_config::Clip,
    pub(crate) drag_start_behavior: incular_core::DragStartBehavior,
    pub(crate) fixed_scroll_offset: Option<f32>,
    pub(crate) allow_implicit_caret_scrolling: bool,
    pub(crate) on_editing_complete: Option<Rc<dyn Fn()>>,
    pub(crate) on_submit: Option<Rc<dyn Fn(String)>>,
    pub(crate) on_tap_outside: Option<EditableOutsideTap>,
    pub(crate) on_tap_up_outside: Option<EditableOutsideTap>,
    pub(crate) on_selection_handle_tapped: Option<Rc<dyn Fn()>>,
    pub(crate) on_app_private_command: Option<AppPrivateCommandCallback>,
    pub(crate) content_insertion: Option<crate::ContentInsertionConfiguration>,
    pub(crate) multiline: bool,
    pub(crate) min_lines: Option<usize>,
    pub(crate) max_lines: Option<usize>,
    pub(crate) expands: bool,
    pub(crate) force_line: bool,
    pub(crate) text_width_basis: incular_text::TextWidthBasis,
    pub(crate) text_height_behavior: Option<TextHeightBehavior>,
    pub(crate) text_align: TextAlign,
    pub(crate) enabled: bool,
    pub(crate) read_only: bool,
    pub(crate) renderer_ignores_pointer: bool,
    pub(crate) platform_tap_selection: bool,
    pub(crate) obscure_text: bool,
    pub(crate) obscuring_character: char,
    pub(crate) keyboard_selection_enabled: bool,
    pub(crate) cursor_width: f32,
    pub(crate) cursor_height: Option<f32>,
    pub(crate) cursor_radius: f32,
    pub(crate) show_cursor: bool,
    pub(crate) paint_cursor_above_text: bool,
    pub(crate) painters: Option<Rc<crate::editable_paint::EditorPainters>>,
    pub(crate) cursor_color: Color,
    pub(crate) mouse_cursor: Option<crate::MouseCursor>,
    pub(crate) background_cursor_color: Option<Color>,
    pub(crate) selection_color: Option<Color>,
    pub(crate) selection_height_style: incular_text::BoxHeightStyle,
    pub(crate) selection_width_style: incular_text::BoxWidthStyle,
    pub(crate) autocorrection_text_rect_color: Option<Color>,
    pub(crate) text_direction: Option<TextDirection>,
    pub(crate) locale: Option<incular_config::Locale>,
    pub(crate) paragraph_layout: bool,
    pub(crate) traits: incular_platform::TextInputTraits,
    pub(crate) scroll_padding: incular_config::EdgeInsets,
    pub(crate) cursor_opacity_animates: bool,
    /// Flutter `spellCheckConfiguration`.
    pub(crate) spell_check: Option<crate::SpellCheckConfiguration>,
    pub(crate) spell_check_suppressed: bool,
    pub(crate) text_magnifier: Option<Rc<crate::magnifier::TextMagnifierState>>,
    pub(crate) stylus_handwriting_enabled: bool,
    pub(crate) has_context_menu_builder: bool,
    pub(crate) legacy_selection_controls: bool,
    pub(crate) toolbar_options: Option<crate::ToolbarOptions>,
    /// A shared portal for the ordinary and spell-check builders.
    pub(crate) context_menu: Option<Rc<crate::context_menu::TextContextMenuState>>,
    /// The field's selection handles, present when it has a handles builder.
    pub(crate) selection_handles:
        Option<Rc<crate::text_selection_handles::TextSelectionHandlesState>>,
}

impl TextFieldSpec {
    pub(crate) fn effective_mouse_cursor(&self) -> crate::MouseCursor {
        self.mouse_cursor.unwrap_or(if self.enabled {
            crate::MouseCursor::Text
        } else {
            crate::MouseCursor::Basic
        })
    }

    pub(crate) fn allows_spell_check_toolbar(&self) -> bool {
        self.enabled
            && !self.read_only
            && !self.spell_check_suppressed
            && (self.has_context_menu_builder || self.selection_handles.is_some())
            && self.spell_check.as_ref().is_some_and(|config| {
                config.spell_check_enabled()
                    && config.misspelled_style().is_some()
                    && config.suggestions_toolbar_builder().is_some()
            })
    }
}

/// Child layouts that transform the incoming constraints or the child size before
/// placing it; some pass through ordinary layout and only affect measurement.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChildLayoutMode {
    /// Tightens a child's width and any separately requested height step.
    IntrinsicWidth {
        step_width: Option<f32>,
        step_height: Option<f32>,
    },
    /// `RenderIntrinsicHeight`: tightens the child height to its intrinsic height.
    IntrinsicHeight,
    /// Declares maximum intrinsic extents while passing through ordinary layout.
    MaxIntrinsicSize {
        maximum: Size,
        minimum: Option<Size>,
    },
    /// Passes through layout without exposing the child’s baseline.
    IgnoreBaseline,
    /// A pure semantics annotation wrapper that leaves layout unchanged.
    SemanticAnnotation,
    /// A pass-through that owns a direct table cell's vertical alignment.
    TableCell(Option<incular_layout::TableCellVerticalAlignment>),
    /// A retained wrapper for the previous-sibling semantics blocker.
    SemanticBlocker,
    /// `RenderRotatedBox`: lays out the child with swapped axes on odd quarter turns.
    QuarterTurns(i32),
}

/// The paragraph options a `Text` states. `None` inherits the ambient
/// `DefaultTextStyle` (Flutter null) and is resolved during lowering. For
/// `max_lines`, `None` inherits and `Some(None)` is an explicit unlimited.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct TextParagraphPolicy {
    pub(crate) align: Option<TextAlign>,
    pub(crate) soft_wrap: Option<bool>,
    pub(crate) max_lines: Option<Option<usize>>,
    pub(crate) overflow: Option<TextOverflow>,
    pub(crate) hyphens: incular_text::Hyphens,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DirectionRequirement {
    pub widget: &'static str,
    pub reason: &'static str,
}

/// Hit-only geometry of a decoration, independent of its paint layers.
#[derive(Clone)]
pub(crate) enum DecorationHitGeometry {
    Box(Rc<crate::BoxDecoration>),
    Shape {
        border: Rc<dyn crate::ShapeBorder>,
        direction: Option<TextDirection>,
    },
}

impl DecorationHitGeometry {
    pub(crate) fn hit_test(&self, size: Size, position: Offset) -> bool {
        match self {
            Self::Box(decoration) => decoration.hit_test(size, position),
            Self::Shape { border, direction } => border.hit_test(
                Rect::from_origin_size(Offset::ZERO, size),
                position,
                *direction,
            ),
        }
    }
}

impl PartialEq for DecorationHitGeometry {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Box(a), Self::Box(b)) => a == b,
            (
                Self::Shape { border: a, direction: direction_a },
                Self::Shape { border: b, direction: direction_b },
            ) => Rc::ptr_eq(a, b) && direction_a == direction_b,
            _ => false,
        }
    }
}

#[derive(Clone)]
pub(crate) enum WidgetKind {
    Box {
        size: Size,
        color: Color,
    },
    Shape {
        path: Arc<Path>,
        fill: Option<Brush>,
        stroke: Option<Box<(Brush, Stroke)>>,
        size: Option<Size>,
    },
    CustomPaint {
        size: Option<Size>,
        display_list: DisplayList,
        painter: Option<crate::painting_effects::PainterDelegate>,
        foreground: bool,
    },
    Colored {
        color: Color,
        is_anti_alias: bool,
        child: Option<Widget>,
    },
    Decorated {
        size: Option<Size>,
        background: Option<Brush>,
        border: Option<Border>,
        radius: CornerRadii,
        hit_decoration: Option<DecorationHitGeometry>,
        child: Widget,
    },
    Banner {
        message: String,
        text_direction: Option<TextDirection>,
        location: crate::utilities::BannerLocation,
        layout_direction: Option<TextDirection>,
        color: Color,
        text_style: Box<TextStyle>,
        shadow: BoxShadow,
        child: Option<Widget>,
    },
    Button(Box<ButtonSpec>),
    Text {
        text: String,
        style: Arc<TextStyle>,
        /// `None` inherits the ambient `DefaultTextStyle` policy (Flutter null).
        align: Option<TextAlign>,
        soft_wrap: Option<bool>,
        /// `None` inherits; `Some(None)` is an explicit unlimited paragraph.
        max_lines: Option<Option<usize>>,
        overflow: Option<TextOverflow>,
        /// Scales the font size, Flutter `Text.textScaler`; `None` follows the
        /// environment text scale (`MediaQuery.textScalerOf`).
        text_scaler: Option<TextScaler>,
        /// Per-span styles of a rich paragraph; `text` is their flattened text.
        spans: Option<Arc<[StyledTextRun]>>,
        /// Mouse annotations indexed by shaped UTF-8 source ranges.
        pointer_spans: Option<Arc<ParagraphMouseSpans>>,
        /// Explicit paragraph direction; `None` resolves against the ambient
        /// `Directionality`, which `Text` requires (Flutter `RichText`).
        text_direction: Option<TextDirection>,
        /// Explicit height behavior; `None` resolves against the ambient
        /// `DefaultTextHeightBehavior`.
        text_height_behavior: Option<TextHeightBehavior>,
        hyphens: incular_text::Hyphens,
        /// Inline `WidgetSpan` boxes of a rich paragraph and the widgets
        /// filling them (Flutter `RichText` children).
        inline: Option<Box<(InlinePlaceholders, Vec<Widget>)>>,
        /// Flutter `Text.strutStyle`.
        strut: Option<Box<StrutStyle>>,
    },
    SelectableText {
        text: String,
        style: Arc<TextStyle>,
        align: TextAlign,
    },
    SelectionArea {
        controller: SelectionAreaController,
        child: Widget,
        /// The area's context menu, present when it has a menu builder.
        context_menu: Option<Rc<crate::context_menu::TextContextMenuState>>,
    },
    SelectionContainer {
        delegate: SelectionContainerDelegate,
        registrar: Option<crate::SelectionRegistrar>,
        child: Widget,
    },
    SelectionListener {
        notifier: SelectionListenerNotifier,
        delegate: SelectionContainerDelegate,
        child: Widget,
    },
    IndexedSemantics {
        index: usize,
        child: Widget,
    },
    SemanticsDebugger {
        label_style: Box<TextStyle>,
        max_nodes: usize,
        child: Widget,
    },
    Image {
        image: Option<ImageHandle>,
        width: Option<f32>,
        height: Option<f32>,
        scale: f32,
        fit: ImageFit,
        repeat: ImageRepeat,
        alignment: crate::AlignmentGeometry,
        sampling: ImageSampling,
        blend_mode: BlendMode,
        center_slice: Option<Rect>,
        match_text_direction: bool,
        source_effects: RasterSourceEffects,
    },
    TextField(Box<TextFieldSpec>),
    Padding {
        padding: EdgeInsets,
        child: Widget,
    },
    Constrained {
        constraints: Constraints,
        child: Option<Widget>,
    },
    Limited {
        max_width: f32,
        max_height: f32,
        child: Option<Widget>,
    },
    Placeholder {
        max_width: f32,
        max_height: f32,
        child: Option<Widget>,
    },
    Overflow {
        min_width: Option<f32>,
        max_width: Option<f32>,
        min_height: Option<f32>,
        max_height: Option<f32>,
        alignment: crate::AlignmentGeometry,
        fit: OverflowBoxFit,
        child: Option<Widget>,
    },
    SizedOverflow {
        size: Size,
        alignment: crate::AlignmentGeometry,
        child: Option<Widget>,
    },
    Unconstrained {
        constrained_axis: Option<Axis>,
        alignment: crate::AlignmentGeometry,
        text_direction: Option<TextDirection>,
        transform: Option<ConstraintMapper>,
        clip_behavior: Clip,
        child: Option<Widget>,
    },
    ProxyLayout {
        mode: ChildLayoutMode,
        child: Option<Widget>,
    },
    Fractional {
        width_factor: Option<f32>,
        height_factor: Option<f32>,
        alignment: crate::AlignmentGeometry,
        child: Option<Widget>,
    },
    Baseline {
        baseline: f32,
        child: Widget,
    },
    RepaintBoundary {
        child: Widget,
    },
    AnimationTicker {
        controller: AnimationController,
        auto_start: bool,
        repeat: Option<bool>,
        revision: Rc<Cell<u64>>,
        retarget: Option<AnimationRetargetBridge>,
        frame: Option<FrameDelegate>,
        child: Widget,
    },
    Gesture {
        behavior: crate::gestures::HitTestBehavior,
        callbacks: Box<GestureCallbacks>,
        child: Widget,
    },
    RawInput {
        // Boxed: the largest widget payload, carried only by input regions.
        kind: Box<RawInputKind>,
        child: Option<Widget>,
    },
    Draggable {
        source: Rc<dyn RetainedDragSource>,
        behavior: crate::gestures::HitTestBehavior,
        child: Widget,
    },
    DragTarget {
        target: Rc<dyn RetainedDragTarget>,
        behavior: crate::gestures::HitTestBehavior,
        child: Widget,
    },
    IgnorePointer {
        ignoring: bool,
        child: Widget,
    },
    AbsorbPointer {
        absorbing: bool,
        child: Widget,
    },
    Align {
        alignment: crate::AlignmentGeometry,
        width_factor: Option<f32>,
        height_factor: Option<f32>,
        child: Option<Widget>,
    },
    Flex {
        axis: Axis,
        main_axis_alignment: MainAxisAlignment,
        main_axis_size: MainAxisSize,
        cross_axis_alignment: CrossAxisAlignment,
        text_direction: Option<TextDirection>,
        vertical_direction: VerticalDirection,
        spacing: f32,
        clip_behavior: Clip,
        children: Vec<Widget>,
    },
    Flexible {
        flex: u32,
        fit: incular_config::FlexFit,
        child: Widget,
    },
    ListBody {
        axis: Axis,
        reverse: bool,
        children: Vec<Widget>,
    },
    OverflowBar {
        spacing: f32,
        overflow_spacing: f32,
        overflow_alignment: WrapCrossAlignment,
        alignment: Option<MainAxisAlignment>,
        overflow_direction: VerticalDirection,
        text_direction: Option<TextDirection>,
        children: Vec<Widget>,
    },
    Wrap {
        axis: Axis,
        alignment: WrapAlignment,
        spacing: f32,
        run_alignment: WrapAlignment,
        run_spacing: f32,
        cross_axis_alignment: WrapCrossAlignment,
        text_direction: Option<TextDirection>,
        vertical_direction: VerticalDirection,
        clip_behavior: Clip,
        children: Vec<Widget>,
    },
    Table {
        columns: usize,
        column_spacing: f32,
        row_spacing: f32,
        column_widths: Vec<(usize, incular_layout::TableColumnWidth)>,
        default_column_width: incular_layout::TableColumnWidth,
        border: Option<Rc<crate::TableBorder>>,
        default_vertical_alignment: incular_layout::TableCellVerticalAlignment,
        text_direction: Option<TextDirection>,
        children: Vec<Widget>,
    },
    Stack {
        alignment: Option<crate::AlignmentGeometry>,
        text_direction: Option<TextDirection>,
        fit: StackFit,
        clip_behavior: Clip,
        children: Vec<Widget>,
    },
    /// `CustomSingleChildLayout`, `CustomMultiChildLayout` and `Flow`.
    CustomLayout {
        delegate: crate::custom_layout::CustomLayoutDelegate,
        children: Vec<Widget>,
    },
    LayoutId {
        id: crate::custom_layout::LayoutChildId,
        child: Widget,
    },
    Positioned {
        left: Option<f32>,
        top: Option<f32>,
        right: Option<f32>,
        bottom: Option<f32>,
        width: Option<f32>,
        height: Option<f32>,
        child: Widget,
    },
    IndexedStack {
        alignment: Option<crate::AlignmentGeometry>,
        text_direction: Option<TextDirection>,
        index: Option<usize>,
        fit: StackFit,
        clip_behavior: Clip,
        children: Vec<Widget>,
    },
    ClipRect {
        clip_behavior: Clip,
        clipper: Option<crate::layout::ClipDelegate>,
        child: Option<Widget>,
    },
    ClipRRect {
        radius: crate::BorderRadius,
        directional_radius: Option<crate::BorderRadiusDirectional>,
        clip_behavior: Clip,
        clipper: Option<crate::layout::ClipDelegate>,
        child: Option<Widget>,
    },
    ClipOval {
        clip_behavior: Clip,
        clipper: Option<crate::layout::ClipDelegate>,
        child: Widget,
    },
    ClipPath {
        path: Option<Arc<Path>>,
        clip_behavior: Clip,
        clipper: Option<crate::layout::ClipDelegate>,
        child: Option<Widget>,
    },
    LayoutBuilder {
        builder: Rc<LayoutBuilderCallback>,
        environment: Option<InheritedScopeValue>,
        environment_boundary: bool,
        /// Optional mutable revision for builders whose callback updates
        /// retained local state without replacing the parent widget.  The
        /// runtime samples this value during layout and rematerializes the
        /// builder child when it changes.
        revision: Option<Rc<Cell<u64>>>,
        /// Whether the child is rebuilt when incoming constraints change.
        /// `false` for builders that only read their `BuildContext`: they
        /// rebuild for a new descriptor, a changed revision or a changed
        /// inherited dependency, never for layout alone.
        constraint_dependent: bool,
        /// State that survives descriptor updates (Flutter's `State`).
        retained: Option<Rc<super::retained_state::RetainedSlot>>,
        /// The fixed child of a static scope (`environment_scope`,
        /// `environment_boundary`), whose builder always returns it. Lets
        /// prevalidation descend through the scope before anything mounts.
        static_child: Option<Widget>,
        /// A composed widget that resolves ambient direction before it can
        /// safely build its render subtree.
        direction_requirement: Option<&'static DirectionRequirement>,
    },
    Visibility {
        visible: bool,
        /// A replacement still lays out and paints, but reports hidden.
        /// Offstage reuses the renderer without providing this scope.
        reported_visible: Option<bool>,
        /// Synthetic sliver wraps can publish a ticker gate without a box scope.
        inherited_ticker: bool,
        hidden: super::HiddenVisibility,
        child: Widget,
    },
    SizeObserver {
        observer: super::SizeObserver,
        child: Widget,
    },
    AspectRatio {
        ratio: f32,
        child: Widget,
    },
    Scroll {
        controller: ScrollController,
        axis: Axis,
        reverse: bool,
        physics: ScrollPhysics,
        increment_calculator: Option<crate::ScrollIncrementCalculator>,
        inherit_physics: Option<bool>,
        keyboard_dismiss: Option<crate::ScrollViewKeyboardDismissBehavior>,
        clip_behavior: Clip,
        drag_start_behavior: DragStartBehavior,
        hit_test_behavior: crate::gestures::HitTestBehavior,
        child: Widget,
    },
    RawScrollbar {
        controller: ScrollController,
        style: RawScrollbarStyle,
        cursor: Option<crate::advanced_scrolling::ScrollbarCursor>,
        shape: Option<crate::advanced_scrolling::raw_scrollbar::ScrollbarThumbShape>,
        child: Widget,
    },
    ListWheelScrollView {
        config: Rc<WheelViewportConfig<Widget>>,
    },
    ListWheelViewport {
        config: Rc<WheelViewportConfig<Widget>>,
    },
    DraggableScrollableSheet {
        config: Rc<DraggableSheetConfig<Widget>>,
    },
    DraggableScrollableActuator {
        actuator: DraggableScrollableActuator,
        child: Widget,
    },
    TwoDimensionalScrollView {
        config: Rc<TwoDimensionalViewportConfig<Widget>>,
    },
    TwoDimensionalViewport {
        config: Rc<TwoDimensionalViewportConfig<Widget>>,
    },
    /// A flow child which remains in the scrolling layout while an inner
    /// compositor transform pins it at the viewport's leading edge.
    PersistentHeader {
        controller: ScrollController,
        axis: Axis,
        reverse: bool,
        pinned: bool,
        child: Widget,
    },
    NotificationListener {
        callback: Option<Rc<dyn Fn(ScrollNotification) -> bool>>,
        child: Widget,
    },
    SliverViewport {
        config: Rc<SliverViewportConfig>,
    },
    Translate {
        controller: TranslationController,
        child: Widget,
    },
    Transform {
        transform: CoreTransform,
        origin: Option<Offset>,
        alignment: Option<Alignment>,
        /// Fractional child-size translation resolved at use time against
        /// the measured size. `None` for absolute transforms; `Some` only
        /// from `FractionalTranslation`, whose fraction cannot resolve
        /// until layout measures the child.
        fraction: Option<Offset>,
        /// Whether pointer hits follow the transform. Painting and
        /// semantics always follow the visual transform.
        transform_hit_tests: bool,
        /// A `Flow` child's slot, whose transform replaces `transform` and
        /// which hides the child when the flow does not paint it.
        slot: Option<crate::custom_layout::FlowSlot>,
        /// Flutter `filterQuality`: when set, the child is rasterized and
        /// resampled through the transform (`ImageFilter.matrix`).
        filter_quality: Option<incular_rendering::FilterQuality>,
        child: Widget,
    },
    /// Compositor-only independent transform parts (motion).
    Affine {
        controller: AffineController,
        child: Widget,
    },
    Scale {
        controller: ScaleController,
        origin: Option<Offset>,
        /// Flutter `filterQuality`: when set, the child is rasterized and
        /// resampled through the transform (`ImageFilter.matrix`).
        filter_quality: Option<incular_rendering::FilterQuality>,
        child: Widget,
    },
    Rotation {
        controller: RotationController,
        origin: Option<Offset>,
        alignment: Option<Alignment>,
        /// Flutter `filterQuality`: when set, the child is rasterized and
        /// resampled through the transform (`ImageFilter.matrix`).
        filter_quality: Option<incular_rendering::FilterQuality>,
        child: Widget,
    },
    FittedBox {
        fit: ImageFit,
        alignment: crate::AlignmentGeometry,
        clip_behavior: Clip,
        child: Option<Widget>,
    },
    Opacity {
        alpha: f32,
        controller: Option<OpacityController>,
        child: Widget,
    },
    Blur {
        sigma_x: f32,
        sigma_y: f32,
        controller: Option<BlurController>,
        enabled: bool,
        child: Widget,
    },
    DropShadow {
        offset: Offset,
        sigma_x: f32,
        sigma_y: f32,
        color: Color,
        controller: Option<DropShadowController>,
        child: Widget,
    },
    ColorFiltered {
        filter: Box<ColorFilter>,
        controller: Option<ColorFilterController>,
        child: Option<Widget>,
    },
    Blend {
        mode: BlendMode,
        child: Widget,
    },
    ShaderMask {
        shader: ShaderCallback,
        blend_mode: BlendMode,
        child: Option<Widget>,
    },
    BackdropFilter {
        blur: GaussianBlur,
        blend_mode: BlendMode,
        enabled: bool,
        matrix: Option<incular_core::Transform>,
        filter_config: Option<Arc<incular_rendering::ImageFilterConfig>>,
        backdrop_key: Option<incular_rendering::BackdropKey>,
        child: Option<Widget>,
    },
    AnnotatedRegion {
        annotation: Annotation,
        sized: bool,
        child: Widget,
    },
    CompositedTransformTarget {
        link: LayerLink,
        child: Widget,
    },
    CompositedTransformFollower {
        link: LayerLink,
        show_when_unlinked: bool,
        offset: Offset,
        target_anchor: LayerAnchor,
        follower_anchor: LayerAnchor,
        child: Widget,
    },
}
