//! Opaque `Widget` transport operations and built-in lowering constructors.
//!
//! Public authoring should normally enter through concrete descriptors; crate-private constructors here are the lowering boundary into `WidgetKind`.

use super::super::*;
use super::{Widget, WidgetAllocation, WidgetChildren, WidgetNode};
use crate::tree::specs::{EditChanged, EditTransform, TextParagraphPolicy};

impl Widget {
    fn from_node(node: WidgetNode) -> Self {
        Self {
            node: Some(Rc::new(WidgetAllocation {
                node,
                validation: Cell::new(0),
            })),
        }
    }

    /// Returns the immutable descriptor node backing this handle.
    #[must_use]
    pub(crate) fn node(&self) -> &WidgetNode {
        &self
            .node
            .as_deref()
            .expect("live widget descriptor handle")
            .node
    }

    #[must_use]
    pub(crate) fn node_mut(&mut self) -> &mut WidgetNode {
        let allocation = Rc::make_mut(self.node.as_mut().expect("live widget descriptor handle"));
        // This also covers mutation while uniquely owned: Rc::make_mut need
        // not clone, but changing keys, kind or children invalidates validation.
        allocation.validation.set(0);
        &mut allocation.node
    }

    /// Success is cached separately with and without ambient Directionality.
    /// A descriptor valid beneath a scope may be invalid beneath a boundary.
    /// Ambient direction only relaxes checks, so success without one also
    /// covers validation beneath a scope.
    pub(in crate::tree) fn validation_cached(&self, has_direction: bool) -> bool {
        let validation = &self
            .node
            .as_ref()
            .expect("live widget descriptor handle")
            .validation;
        let accepted = if has_direction { 0b11 } else { 0b01 };
        validation.get() & accepted != 0
    }

    pub(in crate::tree) fn cache_validation(&self, has_direction: bool) {
        let validation = &self
            .node
            .as_ref()
            .expect("live widget descriptor handle")
            .validation;
        validation.set(validation.get() | (1 << u8::from(has_direction)));
    }

    /// Returns whether two handles refer to the exact same declarative
    /// descriptor allocation.
    ///
    /// Descriptor identity is only an optimization hint; retained identity
    /// still belongs to `Element` and reconciliation continues to honor keys
    /// and widget type compatibility.
    #[must_use]
    pub(crate) fn ptr_eq(&self, other: &Self) -> bool {
        crate::util::same_rc(&self.node, &other.node)
    }

    /// Whether an update can retain this descriptor without walking below
    /// its immediate declarative children.
    ///
    /// Full structural equality is safe here only after every child pair is
    /// known to share its allocation: `Widget::eq` then stops at those child
    /// edges. This admits a fresh wrapper around cached children without
    /// recursively comparing a rebuilt deep tree at every ancestor.
    #[must_use]
    pub(in crate::tree) fn can_skip_update(&self, other: &Self) -> bool {
        if self.ptr_eq(other) {
            return true;
        }
        let current = self.node();
        let incoming = other.node();
        if current.key != incoming.key {
            return false;
        }

        // Eager wheel delegates contain Widget values behind a dynamic child
        // protocol, so children_refs() cannot guard their structural equality.
        // Sharing the immutable config proves equality without visiting those
        // values. Distinct configs take the ordinary reconciliation path.
        match (&current.kind, &incoming.kind) {
            (
                WidgetKind::ListWheelScrollView { config: left },
                WidgetKind::ListWheelScrollView { config: right },
            )
            | (
                WidgetKind::ListWheelViewport { config: left },
                WidgetKind::ListWheelViewport { config: right },
            ) => {
                return Rc::ptr_eq(left, right)
                    && self.semantic_properties() == other.semantic_properties();
            }
            (
                WidgetKind::LayoutBuilder {
                    retained: Some(left),
                    ..
                },
                WidgetKind::LayoutBuilder {
                    retained: Some(right),
                    ..
                },
            ) if left.same_mount_template(right) => {
                return self.semantic_properties() == other.semantic_properties();
            }
            _ => {}
        }

        let children = current.kind.structure().children;
        let other_children = incoming.kind.structure().children;
        children.len() == other_children.len()
            && (children.is_empty()
                || children
                    .into_iter()
                    .zip(other_children)
                    .all(|(child, other_child)| child.ptr_eq(other_child)))
            && current.kind == incoming.kind
            && self.semantic_properties() == other.semantic_properties()
    }

    /// Returns the framework-internal descriptor kind by borrow.
    ///
    /// This is intentionally hidden from the Flutter-facing prelude. Plan 16
    /// removes the remaining workspace consumers that inspect built-in kinds.
    #[must_use]
    pub(crate) fn kind(&self) -> &WidgetKind {
        &self.node().kind
    }

    #[must_use]
    /// Marks a scroll view that set no physics of its own: lowering gives it
    /// the ambient `ScrollBehavior`'s physics, with page snapping on top
    /// when `page` (Flutter's `PageView`).
    pub(crate) fn inherit_scroll_physics(mut self, page: bool) -> Self {
        match self.kind_mut() {
            WidgetKind::Scroll {
                inherit_physics, ..
            } => *inherit_physics = Some(page),
            WidgetKind::SliverViewport { config } => {
                Rc::make_mut(config).inherit_physics = Some(page);
            }
            _ => {}
        }
        self
    }

    pub(crate) fn kind_mut(&mut self) -> &mut WidgetKind {
        &mut self.node_mut().kind
    }

    /// Sets Flutter's `filterQuality` on a transform, scale or rotation
    /// descriptor; other descriptors are returned unchanged.
    #[must_use]
    pub(crate) fn with_transform_filter_quality(
        mut self,
        quality: Option<incular_rendering::FilterQuality>,
    ) -> Self {
        if self.node.is_none() {
            return self;
        }
        match self.kind_mut() {
            WidgetKind::Transform { filter_quality, .. }
            | WidgetKind::Scale { filter_quality, .. }
            | WidgetKind::Rotation { filter_quality, .. } => *filter_quality = quality,
            _ => {}
        }
        self
    }

    #[must_use]
    pub(crate) fn with_transform_hit_geometry(
        mut self,
        pivot: Option<Alignment>,
        enabled: bool,
    ) -> Self {
        if (pivot.is_some() || !enabled)
            && self.node.is_some()
            && let WidgetKind::Transform {
                alignment,
                transform_hit_tests,
                ..
            } = self.kind_mut()
        {
            *alignment = pivot;
            *transform_hit_tests = enabled;
        }
        self
    }

    #[must_use]
    pub(crate) fn semantic_properties(&self) -> &SemanticProperties {
        thread_local! {
            // One shared default for every descriptor that never set semantic
            // or focus metadata, instead of building it on each read.
            static DEFAULT: &'static SemanticProperties = Box::leak(Box::default());
        }
        self.node()
            .semantics
            .as_deref()
            .unwrap_or_else(|| DEFAULT.with(|default| *default))
    }

    #[must_use]
    pub(crate) fn semantic_properties_mut(&mut self) -> &mut SemanticProperties {
        Rc::make_mut(
            self.node_mut()
                .semantics
                .get_or_insert_with(|| Rc::new(SemanticProperties::default())),
        )
    }

    /// Returns typed metadata stored directly on an inherited-scope widget.
    /// This does not walk descendants; callers use it only when a descriptor
    /// contract intentionally wraps its child in a metadata scope.
    pub(crate) fn environment_value<T: Any>(&self) -> Option<&T> {
        match self.kind() {
            WidgetKind::LayoutBuilder {
                environment: Some(scope),
                ..
            } => scope.value.downcast_ref::<T>(),
            _ => None,
        }
    }

    pub(crate) fn with_semantic_locale(mut self, locale: std::sync::Arc<str>) -> Self {
        self.semantic_properties_mut().locale_for_subtree = Some(locale);
        self
    }

    pub(crate) fn set_key(&mut self, key: Option<Key>) {
        self.node_mut().key = key;
    }

    /// Creates a widget from a internal kind descriptor.
    #[must_use]
    pub(crate) fn from_kind(kind: WidgetKind) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind,
            semantics: None,
        })
    }

    /// Attaches retained focus traversal metadata to a transparent wrapper.
    /// This is crate-internal because public callers use the Flutter-shaped
    /// `FocusTraversal*` widgets.
    #[doc(hidden)]
    pub(crate) fn with_focus_traversal_policy(mut self, policy: FocusTraversalPolicyKind) -> Self {
        self.semantic_properties_mut().focus_traversal_policy = Some(policy);
        self
    }

    /// Attaches the `KeepAlive` parent-data request to a transparent wrapper.
    /// Sliver materializers read it to retain the child outside the cache window.
    #[doc(hidden)]
    pub(crate) fn with_ensured_sliver_semantics(mut self) -> Self {
        if !self.semantic_properties().ensures_sliver_semantics {
            self.semantic_properties_mut().ensures_sliver_semantics = true;
        }
        self
    }

    pub(crate) fn with_keep_alive_request(mut self, keep_alive: bool) -> Self {
        if self.semantic_properties().keep_alive != keep_alive {
            self.semantic_properties_mut().keep_alive = keep_alive;
        }
        self
    }

    /// Whether this descriptor asks its sliver to keep it alive.
    #[must_use]
    pub(crate) fn keep_alive_request(&self) -> bool {
        self.semantic_properties().keep_alive
    }

    #[doc(hidden)]
    pub(crate) fn with_focus_traversal_order(
        mut self,
        order: Option<incular_gestures::FocusOrder>,
    ) -> Self {
        let order = order.filter(incular_gestures::FocusOrder::is_valid);
        if order.is_some() || self.node().semantics.is_some() {
            self.semantic_properties_mut().focus_traversal_order = order;
        }
        self
    }

    #[doc(hidden)]
    pub(crate) fn with_excluded_focus(mut self, excluding: bool) -> Self {
        if excluding || self.node().semantics.is_some() {
            self.semantic_properties_mut().exclude_focus = excluding;
        }
        self
    }

    #[doc(hidden)]
    pub(crate) fn with_excluded_focus_traversal(mut self, excluding: bool) -> Self {
        if excluding || self.node().semantics.is_some() {
            self.semantic_properties_mut().exclude_focus_traversal = excluding;
        }
        self
    }

    /// Attaches the retained undo-history capacity to a text-field subtree.
    /// The runtime consumes this metadata without making Widgets depend on
    /// the runtime crate.
    #[doc(hidden)]
    pub(crate) fn with_undo_history_max_entries(mut self, max_entries: usize) -> Self {
        self.semantic_properties_mut().undo_history_max_entries = Some(max_entries);
        self
    }

    /// Marks this retained subtree as the initial focus scope.
    #[doc(hidden)]
    pub(crate) fn with_focus_scope_autofocus(mut self, autofocus: bool) -> Self {
        if autofocus || self.node().semantics.is_some() {
            self.semantic_properties_mut().focus_scope_autofocus = autofocus;
        }
        self
    }

    /// Attaches native text-input hints to the retained editor without making
    /// the renderer-neutral widget depend on a platform window handle.
    #[doc(hidden)]
    pub(crate) fn with_text_input_hints(
        mut self,
        input_type: TextInputTypeHint,
        input_action: TextInputActionHint,
    ) -> Self {
        let semantics = self.semantic_properties_mut();
        semantics.text_input_type = Some(input_type);
        semantics.text_input_action = Some(input_action);
        self
    }

    pub(crate) fn with_gesture_semantic_policy(mut self, policy: GestureSemanticPolicy) -> Self {
        if policy != GestureSemanticPolicy::default() {
            self.semantic_properties_mut().callbacks.gesture_policy = Some(policy);
        }
        self
    }

    #[doc(hidden)]
    pub(crate) fn with_semantic_callback(
        mut self,
        action: SemanticActionKind,
        callback: Rc<dyn Fn() + 'static>,
    ) -> Self {
        let callbacks = &mut self.semantic_properties_mut().callbacks;
        match action {
            SemanticActionKind::Activate => callbacks.activate = Some(callback),
            SemanticActionKind::Increment => callbacks.increment = Some(callback),
            SemanticActionKind::Decrement => callbacks.decrement = Some(callback),
            SemanticActionKind::ScrollForward => callbacks.scroll_forward = Some(callback),
            SemanticActionKind::ScrollBackward => callbacks.scroll_backward = Some(callback),
            SemanticActionKind::LongPress => callbacks.long_press = Some(callback),
            SemanticActionKind::Focus
            | SemanticActionKind::SetText
            | SemanticActionKind::SetSelection
            | SemanticActionKind::ScrollToOffset
            | SemanticActionKind::ShowOnScreen
            | SemanticActionKind::Custom => {}
            SemanticActionKind::ScrollLeft
            | SemanticActionKind::ScrollRight
            | SemanticActionKind::ScrollUp
            | SemanticActionKind::ScrollDown => callbacks.set_directional_scroll(action, callback),
        }
        self
    }

    pub(crate) fn with_custom_semantic_actions(
        mut self,
        bindings: Vec<crate::semantics::CustomActionBinding>,
    ) -> Self {
        let custom = &mut self.semantic_properties_mut().callbacks.custom;
        for binding in bindings {
            if binding.callback.is_none() {
                custom.retain(|existing| {
                    existing.callback.is_some()
                        || existing.action.action() != binding.action.action()
                });
            }
            if let Some(existing) = custom
                .iter_mut()
                .find(|existing| existing.action == binding.action)
            {
                // A descriptor set as a hint does not discard an explicitly
                // registered callback with the same description.
                if binding.callback.is_some() || existing.callback.is_none() {
                    *existing = binding;
                }
            } else {
                custom.push(binding);
            }
        }
        self
    }

    /// Text content when this widget is text-like (DevTools labels only).
    pub fn text_if_any(&self) -> Option<String> {
        match self.kind() {
            WidgetKind::Text { text, .. } | WidgetKind::SelectableText { text, .. } => {
                Some(text.clone())
            }
            _ => None,
        }
    }

    /// Returns the best text label exposed by this widget or a transparent
    /// semantic/layout wrapper around it.
    ///
    /// Control crates use this when a Flutter-shaped control accepts a custom
    /// child instead of a dedicated `label` argument. The visual child and
    /// the accessible name are separate concepts, but text children provide a
    /// safe default for ordinary custom-content controls.
    #[doc(hidden)]
    pub fn semantic_text(&self) -> Option<String> {
        self.semantic_properties()
            .label
            .clone()
            .filter(|label| !label.trim().is_empty())
            .or_else(|| {
                self.semantic_properties()
                    .explicit
                    .as_ref()
                    .and_then(|semantics| semantics.label.clone())
                    .filter(|label| !label.trim().is_empty())
            })
            .or_else(|| widget_text(self))
    }

    #[must_use]
    pub fn box_(size: Size, color: Color) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::Box { size, color },
            semantics: None,
        })
    }
    #[must_use]
    pub(in crate::tree) fn shape(
        path: Arc<Path>,
        fill: Option<Brush>,
        stroke: Option<(Brush, Stroke)>,
        size: Option<Size>,
    ) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::Shape {
                path,
                fill,
                stroke: stroke.map(Box::new),
                size,
            },
            semantics: None,
        })
    }
    #[must_use]
    pub fn decorated(
        size: Option<Size>,
        background: Option<Brush>,
        border: Option<Border>,
        radius: CornerRadii,
        child: Widget,
    ) -> Self {
        Self::decorated_with_hit(size, background, border, radius, child, None)
    }

    pub(crate) fn decorated_with_hit(
        size: Option<Size>,
        background: Option<Brush>,
        border: Option<Border>,
        radius: CornerRadii,
        child: Widget,
        hit_decoration: Option<Rc<crate::BoxDecoration>>,
    ) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::Decorated {
                size,
                background,
                border,
                radius,
                hit_decoration,
                child,
            },
            semantics: None,
        })
    }
    #[must_use]
    pub(crate) fn button(size: Size, color: Color, action: ActionId) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::Button(Box::new(ButtonSpec {
                size,
                color,
                hover_color: None,
                pressed_color: None,
                focused_color: None,
                disabled_color: None,
                enabled: true,
                focusable_when_disabled: false,
                action,
                callback: None,
                hover_action: ActionId(0),
                hover_callback: None,
                exit_action: ActionId(0),
                exit_callback: None,
                has_callback: false,
                child: None,
                interaction: None,
                policy: crate::internal::ActionPolicy::default(),
                mouse_cursor: crate::MouseCursor::Defer,
            })),
            semantics: None,
        })
    }

    /// Builds the retained action surface shared by sibling control crates.
    ///
    /// This is crate-internal plumbing rather than a Flutter-facing widget;
    /// applications should use `GestureDetector` or a Material button.
    #[doc(hidden)]
    pub(crate) fn action_surface(surface: crate::internal::ActionSurface) -> Self {
        let (label, semantic_label) = match surface.content {
            Some(content) => {
                let semantic_label = if surface.label.is_empty() {
                    content.semantic_text()
                } else {
                    Some(surface.label)
                };
                (content, semantic_label)
            }
            None => {
                // The label is both semantic text and visible text here.
                let semantic_label = (!surface.label.is_empty()).then(|| surface.label.clone());
                let label = Widget::padding(
                    surface.padding,
                    Widget::text_styled(surface.label, surface.label_style, TextAlign::Start),
                );
                (label, semantic_label)
            }
        };
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::Button(Box::new(ButtonSpec {
                size: surface.size,
                color: surface.color,
                hover_color: surface.hover_color,
                pressed_color: surface.pressed_color,
                focused_color: surface.focused_color,
                disabled_color: surface.disabled_color,
                enabled: surface.enabled,
                focusable_when_disabled: surface.focusable_when_disabled,
                action: ActionId(0),
                callback: surface.callback,
                hover_action: ActionId(0),
                hover_callback: surface.hover_callback,
                exit_action: ActionId(0),
                exit_callback: surface.exit_callback,
                has_callback: false,
                child: Some(label),
                interaction: surface.interaction,
                policy: surface.policy,
                mouse_cursor: surface.mouse_cursor,
            })),
            // Custom content can be unlabeled; allocate metadata only when a
            // useful accessible name was actually found.
            semantics: semantic_label.map(|label| {
                Rc::new(SemanticProperties {
                    label: Some(label),
                    ..SemanticProperties::default()
                })
            }),
        })
    }
    pub fn bind_callbacks(&mut self, allocate: &mut impl FnMut(Rc<dyn Fn()>) -> ActionId) {
        // `kind_mut` copies a shared descriptor, so only take it when there is
        // something to bind.
        let unbound = matches!(
            self.kind(),
            WidgetKind::Button(spec) if spec.callback.is_some()
                || spec.hover_callback.is_some()
                || spec.exit_callback.is_some()
        );
        if !unbound {
            return;
        }
        if let WidgetKind::Button(spec) = self.kind_mut() {
            if let Some(callback) = spec.callback.take() {
                spec.action = allocate(callback);
                spec.has_callback = true;
            }
            if let Some(callback) = spec.hover_callback.take() {
                spec.hover_action = allocate(callback);
            }
            if let Some(callback) = spec.exit_callback.take() {
                spec.exit_action = allocate(callback);
            }
        }
    }
    #[must_use]
    pub(crate) fn text_styled(text: impl Into<String>, style: TextStyle, align: TextAlign) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::Text {
                text: text.into(),
                style: Arc::new(style),
                align: Some(align),
                soft_wrap: Some(true),
                max_lines: Some(None),
                overflow: Some(TextOverflow::Clip),
                text_direction: None,
                text_height_behavior: None,
                text_scaler: None,
                spans: None,
                inline: None,
                strut: None,
            },
            semantics: None,
        })
    }
    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "text descriptor lowering passes every paragraph field explicitly"
    )]
    pub(crate) fn text_configured(
        text: impl Into<String>,
        style: Arc<TextStyle>,
        paragraph: TextParagraphPolicy,
        text_direction: Option<incular_config::TextDirection>,
        text_height_behavior: Option<incular_text::TextHeightBehavior>,
        text_scaler: Option<TextScaler>,
        spans: Option<Arc<[StyledTextRun]>>,
        inline: Option<Box<(crate::tree::InlinePlaceholders, Vec<Widget>)>>,
        strut: Option<Box<incular_text::StrutStyle>>,
    ) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::Text {
                text: text.into(),
                style,
                align: paragraph.align,
                soft_wrap: paragraph.soft_wrap,
                max_lines: paragraph.max_lines,
                overflow: paragraph.overflow,
                text_direction,
                text_height_behavior,
                text_scaler,
                spans,
                inline,
                strut,
            },
            semantics: None,
        })
    }
    #[must_use]
    pub fn selectable_text_styled(
        text: impl Into<String>,
        style: TextStyle,
        align: TextAlign,
    ) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::SelectableText {
                text: text.into(),
                style: Arc::new(style),
                align,
            },
            semantics: None,
        })
    }
    #[must_use]
    pub fn selection_area(controller: SelectionAreaController, child: Self) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::SelectionArea {
                controller,
                child,
                context_menu: None,
            },
            semantics: None,
        })
    }

    /// A selection area whose secondary click opens the menu `builder`
    /// draws (Flutter's `SelectableRegion.contextMenuBuilder`).
    #[must_use]
    pub fn selection_area_with_context_menu(
        controller: SelectionAreaController,
        child: Self,
        builder: crate::EditableTextContextMenuBuilder,
    ) -> Self {
        crate::context_menu::with_text_context_menu(
            move |state| {
                Self::from_node(WidgetNode {
                    key: None,
                    kind: WidgetKind::SelectionArea {
                        controller: controller.clone(),
                        child: child.clone(),
                        context_menu: Some(state),
                    },
                    semantics: None,
                })
            },
            builder,
        )
    }
    #[must_use]
    pub fn selection_container(delegate: SelectionContainerDelegate, child: Self) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::SelectionContainer { delegate, child },
            semantics: None,
        })
    }
    #[must_use]
    pub(crate) fn selection_listener(notifier: SelectionListenerNotifier, child: Self) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::SelectionListener {
                delegate: SelectionContainerDelegate::with_controller(notifier.controller()),
                notifier,
                child,
            },
            semantics: None,
        })
    }
    #[must_use]
    pub(crate) fn indexed_semantics(index: usize, child: Self) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::IndexedSemantics { index, child },
            semantics: None,
        })
    }
    #[must_use]
    pub(crate) fn semantics_debugger(
        label_style: TextStyle,
        max_nodes: usize,
        child: Self,
    ) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::SemanticsDebugger {
                label_style: Box::new(label_style),
                max_nodes,
                child,
            },
            semantics: None,
        })
    }
    pub(crate) fn semantic_index(&self) -> Option<usize> {
        match self.kind() {
            WidgetKind::IndexedSemantics { index, .. } => Some(*index),
            _ => None,
        }
    }
    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "image lowering passes every image field explicitly"
    )]
    pub(in crate::tree) fn image(
        image: ImageHandle,
        width: Option<f32>,
        height: Option<f32>,
        scale: f32,
        fit: ImageFit,
        repeat: ImageRepeat,
        alignment: Alignment,
        sampling: ImageSampling,
    ) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::Image {
                image,
                width,
                height,
                scale,
                fit,
                repeat,
                alignment,
                sampling,
            },
            semantics: None,
        })
    }
    #[doc(hidden)]
    #[must_use]
    pub fn with_edit_callbacks(
        mut self,
        transform: Option<EditTransform>,
        changed: Option<EditChanged>,
    ) -> Self {
        if let WidgetKind::TextField(spec) = self.kind_mut() {
            spec.edit_transform = transform;
            spec.edit_changed = changed;
        }
        self
    }

    #[must_use]
    pub(crate) fn padding(padding: EdgeInsets, child: Self) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::Padding { padding, child },
            semantics: None,
        })
    }
    /// Creates an explicit retained picture boundary around `child`.
    ///
    /// Descendant paint changes update their own cached picture without
    /// repainting this boundary's otherwise empty retained picture.
    #[must_use]
    pub(crate) fn repaint_boundary(child: Self) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::RepaintBoundary { child },
            semantics: None,
        })
    }
    /// Retains an animation controller in the frame scheduler while `child`
    /// consumes its dependency-tracked value during build.
    #[doc(hidden)]
    #[must_use]
    pub fn animation_ticker(controller: AnimationController, child: impl Into<Self>) -> Self {
        Self::animation_ticker_with_revision(controller, false, Rc::new(Cell::new(0)), child)
    }

    #[doc(hidden)]
    #[must_use]
    pub fn animation_ticker_with_revision(
        controller: AnimationController,
        auto_start: bool,
        revision: Rc<Cell<u64>>,
        child: impl Into<Self>,
    ) -> Self {
        Self::animation_ticker_with_policy(controller, auto_start, None, revision, None, child)
    }

    #[doc(hidden)]
    #[must_use]
    pub fn animation_ticker_with_retarget(
        controller: AnimationController,
        auto_start: bool,
        revision: Rc<Cell<u64>>,
        retarget: Option<AnimationRetargetBridge>,
        child: impl Into<Self>,
    ) -> Self {
        Self::animation_ticker_with_policy(controller, auto_start, None, revision, retarget, child)
    }

    /// Retains a controller as a repeating frame source. `reverse` mirrors
    /// [`AnimationController::repeat`]: when true, every other cycle runs
    /// backwards. The repeat begins on the runtime-provided animation clock,
    /// never at descriptor-construction wall-clock time.
    #[doc(hidden)]
    #[must_use]
    pub fn animation_ticker_repeating_with_retarget(
        controller: AnimationController,
        reverse: bool,
        revision: Rc<Cell<u64>>,
        retarget: Option<AnimationRetargetBridge>,
        child: impl Into<Self>,
    ) -> Self {
        Self::animation_ticker_with_policy(
            controller,
            false,
            Some(reverse),
            revision,
            retarget,
            child,
        )
    }

    /// Ticks `delegate` on the tree's animation clock while it is active,
    /// without rebuilding `child`.
    #[doc(hidden)]
    #[must_use]
    pub fn frame_ticker(delegate: FrameDelegate, child: impl Into<Self>) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::AnimationTicker {
                controller: delegate.idle.clone(),
                auto_start: false,
                repeat: None,
                revision: delegate.revision.clone(),
                retarget: None,
                frame: Some(delegate),
                child: child.into(),
            },
            semantics: None,
        })
    }

    #[doc(hidden)]
    #[must_use]
    fn animation_ticker_with_policy(
        controller: AnimationController,
        auto_start: bool,
        repeat: Option<bool>,
        revision: Rc<Cell<u64>>,
        retarget: Option<AnimationRetargetBridge>,
        child: impl Into<Self>,
    ) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::AnimationTicker {
                controller,
                auto_start,
                repeat,
                revision,
                retarget,
                frame: None,
                child: child.into(),
            },
            semantics: None,
        })
    }
    /// Reports `child`'s laid-out size to `observer` after each layout.
    pub(crate) fn size_observer(observer: impl Fn(Size) + 'static, child: Self) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::SizeObserver {
                observer: super::super::SizeObserver(Rc::new(observer)),
                child,
            },
            semantics: None,
        })
    }
    pub(crate) fn draggable(source: Rc<dyn RetainedDragSource>, child: Self) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::Draggable { source, child },
            semantics: None,
        })
    }
    pub(crate) fn drag_target(target: Rc<dyn RetainedDragTarget>, child: Self) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::DragTarget { target, child },
            semantics: None,
        })
    }
    /// Removes this subtree from pointer hit testing while leaving painting and
    /// semantics intact. Siblings behind it remain eligible for the event.
    #[must_use]
    pub(crate) fn ignore_pointer(ignoring: bool, child: Self) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::IgnorePointer { ignoring, child },
            semantics: None,
        })
    }
    /// Intercepts pointer hit testing at this boundary. Descendants do not
    /// receive ordinary retained interaction while painting and semantics are
    /// preserved.
    #[must_use]
    pub(crate) fn absorb_pointer(absorbing: bool, child: Self) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::AbsorbPointer { absorbing, child },
            semantics: None,
        })
    }
    #[must_use]
    pub(crate) fn layout_builder(
        builder: impl for<'a> Fn(&BuildContext<'a>, Constraints) -> Self + 'static,
    ) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::LayoutBuilder {
                builder: Rc::new(builder),
                environment: None,
                environment_boundary: false,
                revision: None,
                constraint_dependent: true,
                retained: None,
                static_child: None,
                direction_requirement: None,
            },
            semantics: None,
        })
    }

    /// Wraps a child in a retained typed inherited scope. The wrapper is
    /// transparent to layout and paint; descendant builders read the value
    /// through their explicit [`BuildContext`].
    #[must_use]
    pub fn environment_scope<T: Any>(value: T, child: Self) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::LayoutBuilder {
                builder: Rc::new({
                    let child = child.clone();
                    move |_, _| child.clone()
                }),
                environment: Some(InheritedScopeValue::new(value)),
                environment_boundary: false,
                revision: None,
                constraint_dependent: false,
                retained: None,
                static_child: Some(child),
                direction_requirement: None,
            },
            semantics: None,
        })
    }

    pub(crate) fn shared_environment_scope<T: Any>(value: Rc<T>, child: Self) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::LayoutBuilder {
                builder: Rc::new({
                    let child = child.clone();
                    move |_, _| child.clone()
                }),
                environment: Some(InheritedScopeValue::from_shared(value)),
                environment_boundary: false,
                revision: None,
                constraint_dependent: false,
                retained: None,
                static_child: Some(child),
                direction_requirement: None,
            },
            semantics: None,
        })
    }

    /// Creates a transparent retained node that prevents descendants from
    /// reading typed environments installed above it.
    #[must_use]
    pub fn environment_boundary(child: Self) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::LayoutBuilder {
                builder: Rc::new({
                    let child = child.clone();
                    move |_, _| child.clone()
                }),
                environment: None,
                environment_boundary: true,
                revision: None,
                constraint_dependent: false,
                retained: None,
                static_child: Some(child),
                direction_requirement: None,
            },
            semantics: None,
        })
    }

    /// Creates a retained layout builder backed by an explicit local state
    /// revision.  A callback can increment `revision` and the next frame will
    /// rebuild only this builder's child, preserving the rest of the tree.
    /// This is the primitive used by uncontrolled controls such as checkbox,
    /// switch, toggle, and slider.
    #[must_use]
    pub fn stateful_layout_builder(
        revision: Rc<Cell<u64>>,
        builder: impl for<'a> Fn(&BuildContext<'a>, Constraints) -> Self + 'static,
    ) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::LayoutBuilder {
                builder: Rc::new(builder),
                environment: None,
                environment_boundary: false,
                revision: Some(revision),
                constraint_dependent: true,
                retained: None,
                static_child: None,
                direction_requirement: None,
            },
            semantics: None,
        })
    }

    /// Creates a retained builder backed by an explicit local state revision
    /// that, unlike [`Widget::stateful_layout_builder`], ignores layout: its
    /// child is rebuilt when `revision` changes, when the descriptor is
    /// replaced, or when an inherited value it read through `context` changes,
    /// but never because the incoming constraints changed.
    #[must_use]
    /// A builder whose state, created by `init` on first build, survives
    /// descriptor updates at the same place in the tree (Flutter's
    /// `StatefulWidget`). The state receives the builder's revision: bumping
    /// it rebuilds. `constraint_dependent` builders also rebuild when their
    /// constraints change.
    pub(crate) fn retained_builder<S: super::super::RetainedState>(
        init: impl Fn(Rc<Cell<u64>>) -> S + 'static,
        build: impl for<'a> Fn(&Rc<S>, &BuildContext<'a>, Constraints) -> Self + 'static,
        constraint_dependent: bool,
    ) -> Self {
        let revision = Rc::new(Cell::new(0_u64));
        let slot = Rc::new(super::super::retained_state::RetainedSlot::new::<S>(
            revision.clone(),
        ));
        let builder_slot = slot.clone();
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::LayoutBuilder {
                builder: Rc::new(move |context, constraints| {
                    let state = builder_slot.get_or_init(&init);
                    build(&state, context, constraints)
                }),
                environment: None,
                environment_boundary: false,
                revision: Some(revision),
                constraint_dependent,
                retained: Some(slot),
                static_child: None,
                direction_requirement: None,
            },
            semantics: None,
        })
    }

    /// A retained builder whose descriptor may be cloned across independent
    /// mounts. The attached state is created at the element, not shared by
    /// the immutable descriptor. Its builder is not constraint dependent.
    pub(crate) fn mount_scoped_builder<S: super::super::RetainedState>(
        init: impl Fn(Rc<Cell<u64>>) -> S + 'static,
        build: impl for<'a> Fn(&Rc<S>, &BuildContext<'a>) -> Self + 'static,
    ) -> Self {
        use super::super::{context::LayoutBuilderCallback, retained_state::RetainedSlot};
        let init = Rc::new(init);
        let build = Rc::new(build);
        let factory = Rc::new(move |slot: Rc<RetainedSlot>| -> Rc<LayoutBuilderCallback> {
            let init = init.clone();
            let build = build.clone();
            Rc::new(move |context, _| {
                let state = slot.get_or_init(|revision| init(revision));
                build(&state, context)
            })
        });
        let revision = Rc::new(Cell::new(0));
        let slot =
            Rc::new(RetainedSlot::new::<S>(revision.clone()).with_mount_factory(factory.clone()));
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::LayoutBuilder {
                builder: factory(slot.clone()),
                environment: None,
                environment_boundary: false,
                revision: Some(revision),
                constraint_dependent: false,
                retained: Some(slot),
                static_child: None,
                direction_requirement: None,
            },
            semantics: None,
        })
    }

    pub(in crate::tree) fn instantiate_mount_scoped_builder(&self) -> Option<Self> {
        let WidgetKind::LayoutBuilder {
            retained: Some(template),
            ..
        } = self.kind()
        else {
            return None;
        };
        let (slot, instance) = template.mount_instance()?;
        let mut mounted = self.clone();
        let WidgetKind::LayoutBuilder {
            builder,
            revision,
            retained,
            ..
        } = mounted.kind_mut()
        else {
            unreachable!("mount-scoped state belongs to a builder");
        };
        *revision = Some(slot.revision());
        *builder = instance;
        *retained = Some(slot);
        Some(mounted)
    }

    pub fn stateful_builder(
        revision: Rc<Cell<u64>>,
        builder: impl for<'a> Fn(&BuildContext<'a>) -> Self + 'static,
    ) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::LayoutBuilder {
                builder: Rc::new(move |context, _| builder(context)),
                environment: None,
                environment_boundary: false,
                revision: Some(revision),
                constraint_dependent: false,
                retained: None,
                static_child: None,
                direction_requirement: None,
            },
            semantics: None,
        })
    }
    #[must_use]
    pub(crate) fn scroll_view(controller: ScrollController, child: Self) -> Self {
        Self::scroll_view_with_config(
            controller,
            child,
            Axis::Vertical,
            false,
            ScrollPhysics::default(),
        )
    }

    /// Creates a retained viewport with its complete scroll policy.  The
    /// public `scroll_view` helper intentionally keeps its historical
    /// vertical/clamping defaults; scroll descriptors use this crate-local
    /// constructor so axis, reverse, and physics survive lowering.
    #[must_use]
    pub(crate) fn scroll_view_with_config(
        controller: ScrollController,
        child: Self,
        axis: Axis,
        reverse: bool,
        physics: ScrollPhysics,
    ) -> Self {
        Self::scroll_view_with_options(
            controller,
            child,
            axis,
            reverse,
            physics,
            Clip::HardEdge,
            DragStartBehavior::Start,
            crate::gestures::HitTestBehavior::Opaque,
        )
    }

    /// [`Self::scroll_view_with_config`] with Flutter's `clipBehavior` and
    /// `dragStartBehavior` (defaults `Clip.hardEdge` and
    /// `DragStartBehavior.start`).
    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "mirrors the Flutter ScrollView constructor parameters; a config struct would split the lowering path"
    )]
    pub(crate) fn scroll_view_with_options(
        controller: ScrollController,
        child: Self,
        axis: Axis,
        reverse: bool,
        physics: ScrollPhysics,
        clip_behavior: Clip,
        drag_start_behavior: DragStartBehavior,
        hit_test_behavior: crate::gestures::HitTestBehavior,
    ) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::Scroll {
                controller,
                axis,
                reverse,
                physics,
                inherit_physics: None,
                clip_behavior,
                drag_start_behavior,
                hit_test_behavior,
                child,
            },
            semantics: None,
        })
    }
    /// Creates a raw scrollbar with explicit renderer-independent styling.
    #[must_use]
    pub(crate) fn raw_scrollbar_with_style(
        controller: ScrollController,
        style: RawScrollbarStyle,
        child: impl Into<Self>,
    ) -> Self {
        let child = child.into();
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::RawScrollbar {
                controller,
                style,
                child,
            },
            semantics: None,
        })
    }

    /// Creates a retained list-wheel viewport from its focused model.
    #[must_use]
    pub(crate) fn list_wheel_viewport(viewport: ListWheelViewport<Self>) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::ListWheelViewport {
                config: Rc::new(viewport.into_retained_config()),
            },
            semantics: None,
        })
    }

    /// Creates a retained list-wheel scroll view from its focused model.
    #[must_use]
    pub(crate) fn list_wheel_scroll_view(view: ListWheelScrollView<Self>) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::ListWheelScrollView {
                config: Rc::new(view.into_retained_config()),
            },
            semantics: None,
        })
    }

    /// Creates a retained draggable sheet from its focused model.
    #[must_use]
    pub(crate) fn draggable_scrollable_sheet(sheet: DraggableScrollableSheet<Self>) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::DraggableScrollableSheet {
                config: Rc::new(sheet.into_retained_config()),
            },
            semantics: None,
        })
    }

    /// Creates an actuator wrapper which resets the nearest descendant sheets.
    #[must_use]
    pub(crate) fn draggable_scrollable_actuator(
        actuator: DraggableScrollableActuator,
        child: impl Into<Self>,
    ) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::DraggableScrollableActuator {
                actuator,
                child: child.into(),
            },
            semantics: None,
        })
    }

    /// Creates a retained two-dimensional viewport from its focused model.
    #[must_use]
    pub(crate) fn two_dimensional_viewport(viewport: TwoDimensionalViewport<Self>) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::TwoDimensionalViewport {
                config: Rc::new(viewport.into_retained_config()),
            },
            semantics: None,
        })
    }

    /// Creates a retained two-dimensional scroll view from its focused model.
    #[must_use]
    pub(crate) fn two_dimensional_scroll_view(view: TwoDimensionalScrollView<Self>) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::TwoDimensionalScrollView {
                config: Rc::new(view.into_retained_config()),
            },
            semantics: None,
        })
    }
    /// Keeps this flow child at the leading edge of `controller`'s viewport
    /// once it reaches that edge. Consecutive persistent headers push their
    /// predecessors away instead of visually overlapping them.
    #[must_use]
    pub fn persistent_header(controller: ScrollController, child: Self) -> Self {
        Self::persistent_header_with_config(controller, child, Axis::Vertical, false, true)
    }
    /// Creates a persistent header with the axis and direction of its sliver
    /// viewport.  The plain helper retains the historical vertical defaults;
    /// sliver descriptors use this configured form during lowering.
    #[must_use]
    pub(crate) fn persistent_header_with_config(
        controller: ScrollController,
        child: Self,
        axis: Axis,
        reverse: bool,
        pinned: bool,
    ) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::PersistentHeader {
                controller,
                axis,
                reverse,
                pinned,
                child,
            },
            semantics: None,
        })
    }

    /// Creates a retained notification-listener wrapper. The wrapper has no
    /// visual effect; the tree installs the callback on descendant scroll
    /// positions after those positions are mounted.
    pub(crate) fn notification_listener(
        callback: Option<Rc<dyn Fn(ScrollNotification) -> bool>>,
        child: Self,
    ) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::NotificationListener { callback, child },
            semantics: None,
        })
    }

    /// Creates a retained sliver viewport. Sliver children are materialized by
    /// the viewport delegate during layout, so they are not represented as a
    /// declarative `Column` child list.
    #[expect(
        clippy::too_many_arguments,
        reason = "ten Flutter viewport parameters are lowered together; grouping them would only move the list"
    )]
    pub(crate) fn sliver_viewport_with_delegate_options(
        controller: ScrollController,
        axis: Axis,
        reverse: bool,
        physics: ScrollPhysics,
        cache_extent: f32,
        shrink_wrap: bool,
        clip_behavior: Clip,
        drag_start_behavior: DragStartBehavior,
        hit_test_behavior: crate::gestures::HitTestBehavior,
        delegate: Rc<dyn SliverViewportDelegate>,
    ) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::SliverViewport {
                config: Rc::new(SliverViewportConfig {
                    controller,
                    preserve_implicit_scroll_position: false,
                    axis,
                    reverse,
                    text_direction: None,
                    axis_resolution: Default::default(),
                    physics,
                    cache_extent: crate::ScrollCacheExtent::pixels(cache_extent.max(0.)),
                    evict_outside_cache: false,
                    shrink_wrap,
                    clip_behavior,
                    drag_start_behavior,
                    hit_test_behavior,
                    paint_order: Default::default(),
                    delegate,
                    inherit_physics: None,
                    behavior: Default::default(),
                }),
            },
            semantics: None,
        })
    }
    pub(crate) fn with_exact_sliver_cache_window(mut self) -> Self {
        if let WidgetKind::SliverViewport { config } = self.kind_mut() {
            Rc::make_mut(config).evict_outside_cache = true;
        }
        self
    }

    pub(crate) fn with_ambient_sliver_axis(mut self) -> Self {
        if let WidgetKind::SliverViewport { config } = self.kind_mut()
            && config.axis == Axis::Horizontal
        {
            Rc::make_mut(config).axis_resolution =
                crate::scrolling::SliverAxisResolution::Inherited;
        }
        self
    }

    pub(crate) fn with_sliver_cache_extent(
        mut self,
        cache_extent: crate::ScrollCacheExtent,
    ) -> Self {
        if let WidgetKind::SliverViewport { config } = self.kind_mut() {
            Rc::make_mut(config).cache_extent = cache_extent;
        }
        self
    }

    #[must_use]
    pub fn translate(controller: TranslationController, child: Self) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::Translate { controller, child },
            semantics: None,
        })
    }
    /// Applies an arbitrary Kurbo-backed affine transform after layout.
    /// The transform is compositor-only and pivots about the child's center.
    #[must_use]
    pub(crate) fn transform(transform: CoreTransform, child: Self) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::Transform {
                transform,
                origin: None,
                alignment: None,
                fraction: None,
                transform_hit_tests: true,
                slot: None,
                filter_quality: None,
                child,
            },
            semantics: None,
        })
    }
    /// Applies an arbitrary Kurbo-backed affine transform after layout,
    /// about `origin`. The transform is compositor-only.
    #[must_use]
    pub(crate) fn transform_around(transform: CoreTransform, origin: Offset, child: Self) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::Transform {
                transform,
                origin: Some(Offset::finite_or_zero(origin)),
                alignment: None,
                fraction: None,
                transform_hit_tests: true,
                slot: None,
                filter_quality: None,
                child,
            },
            semantics: None,
        })
    }
    /// Applies a child-size fractional translation after layout, retaining
    /// the fraction and hit-test policy instead of freezing pixels at
    /// conversion time, when the child size is not yet known.
    #[must_use]
    pub(crate) fn fractional_translation(
        fraction: Offset,
        transform_hit_tests: bool,
        child: Self,
    ) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::Transform {
                transform: CoreTransform::IDENTITY,
                origin: None,
                alignment: None,
                fraction: Some(fraction),
                transform_hit_tests,
                slot: None,
                filter_quality: None,
                child,
            },
            semantics: None,
        })
    }
    #[must_use]
    pub(crate) fn fitted_box(
        fit: ImageFit,
        alignment: crate::AlignmentGeometry,
        clip_behavior: Clip,
        child: Option<Self>,
    ) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::FittedBox {
                fit,
                alignment,
                clip_behavior,
                child,
            },
            semantics: None,
        })
    }
    /// A compositor-only transform whose parts follow `controller`.
    #[must_use]
    pub fn controlled_affine(controller: AffineController, child: Self) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::Affine { controller, child },
            semantics: None,
        })
    }
    #[must_use]
    pub fn controlled_scale(controller: ScaleController, child: Self) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::Scale {
                controller,
                origin: None,
                filter_quality: None,
                child,
            },
            semantics: None,
        })
    }
    #[must_use]
    pub fn controlled_rotation(controller: RotationController, child: Self) -> Self {
        Self::controlled_rotation_with_alignment(controller, None, child)
    }
    #[must_use]
    pub(crate) fn controlled_rotation_with_alignment(
        controller: RotationController,
        alignment: Option<Alignment>,
        child: Self,
    ) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::Rotation {
                controller,
                origin: None,
                alignment,
                filter_quality: None,
                child,
            },
            semantics: None,
        })
    }
    #[must_use]
    pub(crate) fn opacity(alpha: f32, child: Self) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::Opacity {
                alpha: normalize_opacity(alpha),
                controller: None,
                child,
            },
            semantics: None,
        })
    }
    #[must_use]
    pub fn controlled_opacity(controller: OpacityController, child: Self) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::Opacity {
                alpha: controller.opacity(),
                controller: Some(controller),
                child,
            },
            semantics: None,
        })
    }
    #[must_use]
    pub(crate) fn blur(sigma: f32, child: Self) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::Blur {
                sigma_x: normalize_sigma(sigma),
                sigma_y: normalize_sigma(sigma),
                controller: None,
                child,
            },
            semantics: None,
        })
    }
    #[must_use]
    pub fn asymmetric_blur(sigma_x: f32, sigma_y: f32, child: Self) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::Blur {
                sigma_x: normalize_sigma(sigma_x),
                sigma_y: normalize_sigma(sigma_y),
                controller: None,
                child,
            },
            semantics: None,
        })
    }
    #[must_use]
    pub fn controlled_blur(controller: BlurController, child: Self) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::Blur {
                sigma_x: controller.sigma(),
                sigma_y: controller.sigma(),
                controller: Some(controller),
                child,
            },
            semantics: None,
        })
    }
    #[must_use]
    pub(crate) fn drop_shadow(offset: Offset, sigma: f32, color: Color, child: Self) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::DropShadow {
                offset: Offset::finite_or_zero(offset),
                sigma_x: normalize_sigma(sigma),
                sigma_y: normalize_sigma(sigma),
                color,
                controller: None,
                child,
            },
            semantics: None,
        })
    }
    #[must_use]
    pub fn controlled_drop_shadow(controller: DropShadowController, child: Self) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::DropShadow {
                offset: controller.offset(),
                sigma_x: controller.sigma(),
                sigma_y: controller.sigma(),
                color: controller.color(),
                controller: Some(controller),
                child,
            },
            semantics: None,
        })
    }
    #[must_use]
    pub(crate) fn color_filtered(filter: ColorFilter, child: Self) -> Self {
        Self::color_filtered_with_child(filter, None, Some(child))
    }
    #[must_use]
    pub(crate) fn color_filtered_with_child(
        filter: ColorFilter,
        controller: Option<ColorFilterController>,
        child: Option<Self>,
    ) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::ColorFiltered {
                filter: Box::new(filter),
                controller,
                child,
            },
            semantics: None,
        })
    }
    #[must_use]
    pub fn color_matrix(filter: ColorFilter, child: Self) -> Self {
        Self::color_filtered(filter, child)
    }
    #[must_use]
    pub fn controlled_color_filtered(controller: ColorFilterController, child: Self) -> Self {
        Self::color_filtered_with_child(controller.filter(), Some(controller), Some(child))
    }
    #[must_use]
    pub(crate) fn blend(mode: BlendMode, child: Self) -> Self {
        Self::from_node(WidgetNode {
            key: None,
            kind: WidgetKind::Blend { mode, child },
            semantics: None,
        })
    }
    #[must_use]
    pub fn with_key(mut self, key: impl Into<Key>) -> Self {
        self.set_key(Some(key.into()));
        self
    }
    /// Overrides the accessible label contributed by this meaningful widget.
    #[must_use]
    pub fn accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.semantic_properties_mut().label = Some(label.into());
        self
    }
    /// Adds a screen-reader description without changing visible text.
    #[must_use]
    pub fn accessibility_description(mut self, description: impl Into<String>) -> Self {
        self.semantic_properties_mut().description = Some(description.into());
        self
    }
    /// Supplies explicit Incular semantic metadata for this visual widget.
    /// This is useful for icon-only controls, meaningful images, headings,
    /// dialogs, and custom-painted controls; it never exposes native adapter
    /// types to application code.
    #[must_use]
    pub fn semantics(mut self, semantics: ExplicitSemantics) -> Self {
        self.semantic_properties_mut().explicit = Some(Box::new(semantics));
        self
    }
    /// Flutter's `alwaysIncludeSemantics` for an opacity widget. By default a
    /// zero-alpha opacity hides its child's semantics, as Flutter does.
    #[must_use]
    pub fn always_include_semantics(mut self, include: bool) -> Self {
        self.semantic_properties_mut().always_include_semantics = include;
        self
    }
    /// Excludes this widget and its implementation-detail subtree from semantics.
    #[must_use]
    pub fn exclude_semantics(mut self) -> Self {
        self.semantic_properties_mut().hidden = true;
        self
    }
    /// Merges meaningful descendants into one logical accessible node. The
    /// widget itself must have explicit semantics or a meaningful built-in
    /// role; descendants are not exposed separately.
    #[must_use]
    pub fn merge_semantics(mut self) -> Self {
        self.semantic_properties_mut().merge_descendants = true;
        self
    }
    /// Suppresses preceding semantic siblings at this stacking level. Use it
    /// for a modal/dialog region so screen-reader traversal cannot fall through
    /// to visual content behind the active modal.
    #[must_use]
    pub fn block_semantics(mut self) -> Self {
        self.semantic_properties_mut().block_previous_siblings = true;
        self
    }
    #[must_use]
    pub fn key(&self) -> Option<&Key> {
        self.node().key.as_ref()
    }
    /// Stable diagnostic name for the concrete widget represented by this
    /// opaque transport value.
    ///
    /// This intentionally exposes no layout/render taxonomy or payload data.
    #[must_use]
    pub fn debug_type_name(&self) -> &'static str {
        self.type_().name()
    }
    /// Flutter's `Widget.canUpdate`: the same widget type and key, so an
    /// element can take the other descriptor.
    pub(crate) fn can_update(&self, other: &Self) -> bool {
        self.type_() == other.type_() && self.key() == other.key()
    }
    pub(in crate::tree) fn type_(&self) -> WidgetType {
        self.kind().structure().widget_type
    }
    /// Shallow child view for reconciliation. Deep per-child clones were the
    /// measured allocation fire on wide trees (Task 15); reconciliation only
    /// needs references because cloning happens once per *created* element.
    pub(in crate::tree) fn children_refs(&self) -> WidgetChildren<'_> {
        self.kind().structure().children
    }
}

impl RawScrollbar {
    /// Lowers this stateful scrollbar model into a retained widget overlay.
    #[must_use]
    pub fn into_widget(self, child: impl Into<Widget>) -> Widget {
        Widget::raw_scrollbar_with_style(self.controller(), self.style(), child)
    }

    /// Alias for [`Self::into_widget`] with Flutter-style wrapper wording.
    #[must_use]
    pub fn with_child(self, child: impl Into<Widget>) -> Widget {
        self.into_widget(child)
    }
}

impl From<RawScrollbar> for Widget {
    fn from(value: RawScrollbar) -> Self {
        value.into_widget(Widget::box_(Size::ZERO, Color::TRANSPARENT))
    }
}

impl From<ListWheelViewport<Widget>> for Widget {
    fn from(value: ListWheelViewport<Widget>) -> Self {
        Widget::list_wheel_viewport(value)
    }
}

impl From<ListWheelScrollView<Widget>> for Widget {
    fn from(value: ListWheelScrollView<Widget>) -> Self {
        Widget::list_wheel_scroll_view(value)
    }
}

impl From<DraggableScrollableSheet<Widget>> for Widget {
    fn from(value: DraggableScrollableSheet<Widget>) -> Self {
        Widget::draggable_scrollable_sheet(value)
    }
}

impl DraggableScrollableActuator {
    /// Lowers this reset channel into a transparent retained wrapper.
    #[must_use]
    pub fn with_child(self, child: impl Into<Widget>) -> Widget {
        Widget::draggable_scrollable_actuator(self, child)
    }
}

impl From<DraggableScrollableActuator> for Widget {
    fn from(value: DraggableScrollableActuator) -> Self {
        value.with_child(Widget::box_(Size::ZERO, Color::TRANSPARENT))
    }
}

impl From<TwoDimensionalViewport<Widget>> for Widget {
    fn from(value: TwoDimensionalViewport<Widget>) -> Self {
        Widget::two_dimensional_viewport(value)
    }
}

impl From<TwoDimensionalScrollView<Widget>> for Widget {
    fn from(value: TwoDimensionalScrollView<Widget>) -> Self {
        Widget::two_dimensional_scroll_view(value)
    }
}
