//! Declarative widgets and the retained element/render tree.
//!
//! The public crate root provides canonical first-class Flutter-style widget descriptors
//! while internal retained tree execution remains Rust-native.
//!
//! Author UI with concrete widget types and erase to [`Widget`] only at composition or
//! framework boundaries:
//!
//! ```
//! use incular_widgets::{Column, Padding, Text, Widget};
//!
//! let screen: Widget = Padding::all(
//!     16.0,
//!     Column::new([
//!         Widget::from(Text::new("Title")),
//!         Widget::from(Text::new("Body")),
//!     ]),
//! )
//! .into();
//!
//! assert_eq!(screen.debug_type_name(), "Padding");
//! ```
//!
//! `Widget` is intentionally an opaque transport value. Application-specific widgets should
//! normally be ordinary Rust components/functions that compose these public descriptors.
//! Custom drawing uses [`CustomPaint`] / [`CustomPainter`]. The retained built-in taxonomy is
//! sealed; Incular does not expose `WidgetKind` or a public trait-object render protocol.

mod macros;

mod advanced_scrolling;
mod advanced_slivers;
mod animated_cross_fade;
mod animated_size;
mod animated_switcher;
mod animation_transitions;
mod app_shell;
mod asset_bundle;
mod async_builders;
mod back_gesture;
mod box_fit;
mod compositing;
mod context_menu;
mod custom_layout;
#[cfg(feature = "devtools")]
#[doc(hidden)]
pub mod devtools;
#[cfg(feature = "devtools")]
pub(crate) mod devtools_edit;
#[cfg(feature = "devtools")]
pub(crate) mod devtools_props;
#[cfg(feature = "devtools")]
mod devtools_trees;
mod dismissible;
mod display_feature_sub_screen;
mod drag_boundary;
mod drag_drop;
mod dx;
mod environment;
mod explicit_animations;
pub mod extensions;
mod stateful_builder;
mod text_selection_handles;
pub use dx::motion;
pub use dx::transition::{TransitionExt, Transitionable};
mod external_drop;
mod fade_in_image;
mod fast_hash;
mod flutter_logo;
mod focus_keyboard;
mod forms;
mod gestures;
mod hero;
mod implicit_animations;
mod indexed_semantics;
mod inherited_data;
mod interactive_viewer;
#[doc(hidden)]
pub mod internal;
mod layout;
mod magnifier;
mod navigation;
mod navigation_scopes;
mod notification;
mod overscroll_indicator;
mod painting_effects;
mod platform_view;
mod platform_widgets;
mod provider_image;
mod radio_selection;
mod raw_input;
mod raw_menu_anchor;
mod raw_tooltip;
mod recursion;
mod render_object;
mod repeating_animation_builder;
mod safe_area;
mod scroll_aware_image_provider;
mod scroll_behavior;
mod scrolling;
mod selection;
mod selection_container;
mod selection_listener;
mod semantics;
mod semantics_debugger;
mod shortcut_registry;
mod size_changed;
mod spell_check;
mod transient;
mod text_formatters;
mod tree;
mod tweens;
mod util;
mod utilities;
pub mod widget_previews;

// Re-export only the Flutter Widgets vocabulary.  Retained-tree IDs,
// renderer diagnostics, and Incular-only helpers stay private to this crate;
// the facade's `internal` module is the sole explicitly documented bridge for
// sibling implementation crates.
pub use advanced_scrolling::{
    CacheExtentStyle, ChangeReportingBehavior, ChildVicinity, DiagonalDragBehavior,
    DraggableAnimationStep, DraggableExtentError, DraggableNotificationSubscription,
    DraggableScrollableActuator, DraggableScrollableController, DraggableScrollableNotification,
    DraggableScrollableSheet, DraggableScrollableState, DraggableSheetDelta, DraggableSheetExtent,
    DraggableSizeAnimation, DraggableSnap, DraggableSnapTarget, FixedExtentScrollController,
    ListWheelScrollView, ListWheelViewport, RawScrollbar, RawScrollbarGeometry,
    RawScrollbarOrientation, RawScrollbarStyle, TwoDimensionalChildDelegate,
    TwoDimensionalChildLayout, TwoDimensionalConfigError, TwoDimensionalConstraints,
    TwoDimensionalScrollDelta, TwoDimensionalScrollView, TwoDimensionalScrollable,
    TwoDimensionalViewport, TwoDimensionalViewportLayout, WheelChildDelegate, WheelChildLayout,
    WheelConfigError, WheelLayout, WheelMatrix, WheelProjection,
};
pub use advanced_slivers::{
    AnimatedGrid, AnimatedGridController, AnimatedItem, AnimatedItemBuilder, AnimatedItemPhase,
    AnimatedList, AnimatedListController, AnimatedRemovedItemBuilder, AutomaticKeepAlive,
    KeepAlive, KeepAliveHandle, KeepAliveNotification, KeepAliveRegistry, SliverAnimatedGrid,
    SliverAnimatedGridController, TreeRowAnimation, TreeSliver, TreeSliverController,
    TreeSliverIndentation, TreeSliverNode, TreeSliverNodeId,
};
pub use animated_cross_fade::{AnimatedCrossFade, AnimatedCrossFadeBuilder, CrossFadeState};
pub use animated_size::AnimatedSize;
pub use animated_switcher::{
    AnimatedSwitcher, AnimatedSwitcherAnimatedTransitionBuilder, AnimatedSwitcherLayoutBuilder,
    AnimatedSwitcherTransitionBuilder,
};
pub use animation_transitions::{
    AlignTransition, AnimatedTransitionBuilder, DecoratedBoxTransition, Decoration,
    DefaultTextStyleTransition, DualTransitionBuilder, MatrixTransition, PositionedTransition,
    RelativePositionedTransition, RelativeRect, RelativeRectTween, SizeTransition, TweenAnimation,
};
pub use app_shell::{
    ApplicationBootstrapHost, ApplicationBootstrapOptions, ApplicationBootstrapSpec,
    AuxiliaryViewError, AuxiliaryViewHandle, AuxiliaryViewHost, AuxiliaryViewOutcome,
    AuxiliaryViewRequest, BackButtonDispatcher, BackCallbackSubscription, BasicRouterDelegate,
    ChildBackButtonDispatcher, ClosureRouteInformationParser, MemoryRouteInformationProvider,
    NavigationNotification, NavigationNotificationKind, NavigationNotificationListener,
    NavigationNotificationSubscription, NoopAuxiliaryViewHost, NoopWindowChromeSink,
    RootBackButtonDispatcher, RouteInformation, RouteInformationListener, RouteInformationParser,
    RouteInformationProvider, RouteInformationReportingType, RouteInformationSubscription, Router,
    RouterConfig, RouterData, RouterDelegate, RouterDelegateListener, RouterDelegateSubscription,
    RouterError, StringRouteInformationParser, Title, TitleController, TitleData, TitleError, View,
    ViewAnchor, ViewAnchorController, ViewAnchorData, ViewAnchorSubscription, ViewController,
    ViewData, ViewEvent, ViewId, ViewLifecycle, ViewMetrics, ViewSubscription, WidgetsApp,
    WidgetsAppController, WidgetsAppData, WindowChromeSink, WindowDragRegion, WindowResizeRegion,
    normalize_route_location,
};
pub use asset_bundle::DefaultAssetBundle;
pub use async_builders::{
    AsyncSnapshot, AsyncWidgetBuilder, ConnectionState, FutureBuilder, StreamBuilder,
    StreamBuilderBase, StreamEvent, UiCompleter, UiFuture, UiStream, UiStreamController,
    UiStreamSubscription,
};
pub use back_gesture::{
    BackGestureDispatch, BackGestureNotification, BackGestureObserver, BackGestureObserverFailure,
    BackGestureObserverRegistration, add_back_gesture_observer, add_shared_back_gesture_observer,
    dispatch_back_gesture, dispatch_back_gesture_checked, dispatch_pop_route, dispatch_pop_route_checked,
};
pub use box_fit::{FittedSizes, apply_box_fit};
pub use incular_layout::position_dependent_box;
pub use compositing::{
    AnnotatedRegion, BackdropFilter, BackdropGroup, CompositedTransformFollower,
    CompositedTransformTarget, ShaderCallback, ShaderMask,
};
#[doc(hidden)]
pub use context_menu::TextContextMenuState;
pub use context_menu::{
    ContextMenuButtonItem, ContextMenuButtonType, DesktopTextSelectionToolbarLayoutDelegate,
    EditableTextContextMenuBuilder, TextContextMenu, TextSelectionToolbarAnchors,
    TextSelectionToolbarLayoutDelegate,
};
pub use custom_layout::{
    CustomMultiChildLayout, CustomSingleChildLayout, Flow, FlowDelegate, FlowPaintError,
    FlowPaintingContext, IgnoreBaseline, LayoutChildError, LayoutChildId, LayoutId, LayoutRelayout, MissingLayoutChild,
    MultiChildLayoutContext, MultiChildLayoutDelegate, PositionedDirectional,
    SingleChildLayoutDelegate,
};
pub use dismissible::{DismissDirection, DismissUpdateDetails, Dismissible};
pub use display_feature_sub_screen::DisplayFeatureSubScreen;
pub use drag_boundary::{DragBoundary, DragBoundaryDelegate, RectDragBoundary};
pub use drag_drop::{
    DragAnchorStrategy, DragDropContext, DragTarget, Draggable, DraggableDetails,
    LongPressDraggable,
};
pub use environment::{
    ContentSensitivity, DefaultSelectionStyle, DefaultTextHeightBehavior, DefaultTextStyle,
    Directionality, IconTheme, Localizations, LookupBoundary, MediaQuery, MediaQueryData,
    MediaQueryEdges, Orientation, OrientationBuilder, PrimaryScrollController, ScrollConfiguration,
    SensitiveContent, SensitiveContentHost, TickerMode,
};
pub use explicit_animations::{AnimatedBuilder, Listenable, ListenableBuilder, TransitionBuilder};
pub use fade_in_image::FadeInImage;
pub use flutter_logo::{FlutterLogo, FlutterLogoLayout, FlutterLogoStyle};
pub use incular_gestures::{
    FocusExtent, FocusKeyEventHandler, FocusRawKeyEventHandler, FocusRect, RawKeyEvent,
    TapCallbackError, TapCallbackErrorHandler, TapCallbackKind, TapCancelReason, TraversalDirection, TraversalEdgeBehavior,
    set_tap_callback_error_handler,
};
#[doc(hidden)]
pub use incular_gestures::RawKeyDelivery;
pub use focus_keyboard::{
    Action, ActionContext, ActionDispatchInvocation, ActionDispatcher, ActionInvocationPhase, ActionListener,
    ActionLookupError, ActionOutput, ActionResult, ActionScope, Actions, CallbackAction, CallbackShortcuts,
    Command, CommandId, DirectionalFocusIntent, ExcludeFocus, ExcludeFocusTraversal, Focus, FocusManager, FocusNode,
    FocusOrder, FocusScope, FocusScopeNode, FocusScopeSubscription, FocusTraversalGroup,
    FocusTraversalOrder, FocusTraversalPolicy, FocusTraversalPolicyKind, FocusableActionDetector,
    Intent, KeyEventResult, KeyboardListener, LockState, LogicalKeySet, LogicalShortcutKey,
    OrderedTraversalPolicy, ReadingOrderTraversalPolicy, ScopedActionDispatcher, ShortcutActivator,
    ShortcutKey, ShortcutScope, ShortcutTrigger, Shortcuts, SingleActivator, VoidCallbackAction, VoidCallbackIntent,
    WidgetOrderTraversalPolicy,
    UnfocusDisposition,
};
pub use forms::{
    AutocompleteFieldViewBuilder, AutocompleteHighlightedOption, AutocompleteOnSelected,
    AutocompleteOptionsBuilder, AutocompleteOptionsViewBuilder, AutofillGroup, AutovalidateMode,
    CallbackTextInputFormatter,
    FilteringTextInputFormatter, Form, FormController, FormField, FormFieldState, FormState,
    GenericFormField, LengthLimitingTextInputFormatter, MaxLengthEnforcement, RawAutocomplete,
    TextInputFormatter, UndoHistory,
};
pub use gestures::{
    AbsorbPointer, DragDownDetails, DragEndDetails, DragStartDetails, DragUpdateDetails,
    ForcePressDetails, GestureDetector, HitTestBehavior, IgnorePointer, TapUpDetails,
};
pub use hero::{
    CreateRectTween, Hero, HeroFlightDirection, HeroFlightShuttleBuilder, HeroMode,
    HeroPlaceholderBuilder, HeroRectTween, HeroShuttleContext, HeroTag,
};
pub use implicit_animations::{
    AlignmentGeometry, AnimatedAlign, AnimatedContainer, AnimatedDefaultTextStyle,
    AnimatedFractionallySizedBox, AnimatedOpacity, AnimatedPadding, AnimatedPhysicalModel,
    AnimatedPositioned, AnimatedPositionedDirectional, AnimatedRotation, AnimatedScale,
    AnimatedSlide, EdgeInsetsGeometry, TweenAnimationBuilder, TweenAnimationWidgetBuilder,
};
pub use incular_animation::{
    Animatable, AnimatableExt, Animation, AnimationController, AnimationSource, AnimationStatus,
    AnimationStyle, CompoundAnimation, Curve, CurveTween, CurvedAnimation, Curves, DoubleAnimation,
    DrivenAnimation, LinearTween, ProxyAnimation, ReverseAnimation, Ticker, TickerFuture, Tween,
    always_complete, always_dismissed, disable_animations,
};
pub use incular_config::{
    Alignment, AlignmentDirectional, Axis, AxisDirection, Brightness, Clip, CrossAxisAlignment,
    FlexFit, FractionalOffset, MainAxisAlignment, MainAxisSize, StackFit, TextDirection,
    TransientPresentation, TransientRole, VerticalDirection, WrapAlignment, WrapCrossAlignment,
};
pub use incular_core::{Color, Key, LocalKey, Offset, Rect, Size, UniqueKey, ValueKey};
pub use incular_gestures::{FocusHighlightManager, FocusHighlightMode, FocusHighlightStrategy};
pub use incular_image::{
    AnyImageProvider, AssetBundleImageKey, AssetImage, ExactAssetImage, FileImage, ImageAssetBundle,
    ImageChunkEvent, ImageConfiguration, ImageLoadState, ImageProvider, ImageResizeOptions,
    ImageStream, ImageStreamCompleter, MemoryImage, ResizeImage, ResizeImageKey, ResizeImagePolicy,
};
pub use incular_platform::autofill::AutofillContextAction;
pub use incular_platform::{SystemSound, SystemSoundType};
pub use incular_platform::{
    BackGestureEvent, DefaultSpellCheckService, PlatformViewCommand, PlatformViewCompositing,
    PlatformViewId, PlatformViewPointerEvent, PredictiveBackEvent, SmartDashesType,
    SmartQuotesType, SpellCheckCallback,
    SpellCheckResults, SpellCheckService, SuggestionSpan, SwipeEdge, SystemUiOverlayStyle,
};
pub use incular_rendering::{
    Annotation, BlendMode, Canvas, ColorFilter, FilterQuality, LayerAnchor, LayerLink,
    LinearGradient, Paint, PaintStyle, Path, RRect, RadialGradient, Shader, Shadow, SweepGradient,
};
pub use incular_scroll::{
    ClampingScrollPhysics, PageMetrics, PageMetricsUpdate, ScrollAnimation, ScrollAnimationCompletion, ScrollController,
    ScrollMetrics, ScrollNotification,
    ScrollNotificationSubscription, ScrollNotificationType, ScrollPhysics,
    ScrollViewKeyboardDismissBehavior, SliverConstraints, SliverGrowthDirection,
};
pub use incular_semantics::{
    AccessibilityFocusBlockType, SemanticValidationResult, SemanticsInputType,
};
pub use incular_text::{
    FontFeature, FontVariation, FontWeight, IconData, RichText, StrutStyle, TextAlign,
    TextBaseline, TextEditingController, TextEditingValue, TextHeightBehavior, TextMetricAdjustment,
    TextOverflow, TextRange, TextScaler, TextSelection, TextSpan, TextStyle, TextStyleApply,
    TextWidthBasis, WidgetSpan, lerp_font_variations,
};
pub use indexed_semantics::IndexedSemantics;
pub use scrolling::{ClipOverlapBehavior, SliverClipRRect, SliverClipRect};
pub use inherited_data::{
    CapturedThemes, InheritedModel, InheritedModelData, InheritedNotifier, SharedAppData,
    SharedAppDataKey, SharedAppDataStore,
};
pub use interactive_viewer::{InteractiveViewer, PanAxis, TransformationController};
pub use layout::{
    Align, AspectRatio, Baseline, Builder, Center, ClipOval, ClipPath, ClipRRect, ClipReclip,
    ClipRect, ClipShape, ColoredBox, Column, ConstrainedBox, ConstraintsTransformBox, Container,
    CustomClipper, CustomPaint, Expanded, FittedBox, Flex, Flexible, FractionalTranslation,
    FractionallySizedBox, IndexedStack, IntrinsicHeight, IntrinsicWidth, KeyedSubtree,
    LayoutBuilder, LimitedBox, MaxIntrinsicSize, NavigationToolbar, Offstage, OverflowBar,
    OverflowBarAlignment,
    OverflowBox, OverflowBoxFit, Padding, Placeholder, Positioned, PreferredSize, RepaintBoundary,
    RotatedBox, Row, SizedBox, SizedOverflowBox, Spacer, Stack, Table, TableCell,
    TableCellVerticalAlignment, TableColumnWidth, TableRow, TableRowLengthError, UnconstrainedBox,
    Visibility, Wrap,
};
pub use magnifier::{
    MagnifierBuilder, MagnifierController, MagnifierDecoration, MagnifierInfo, MagnifierScaleError,
    RawMagnifier, TextMagnifierConfiguration,
};
/// The navigation-scope dispatcher is kept under an explicit name so a future
/// app-shell `BackButtonDispatcher` can coexist in this facade without a
/// duplicate public binding.
pub use navigation::BackButtonDispatcher as NavigationBackButtonDispatcher;
pub use navigation::{
    AnimatedModalBarrier, BackButtonListener, BackDispatchReport, BackHandlerResult,
    BackRegistration, ModalBarrier, NavigatorPopHandler, NavigatorPopHandlerController,
    OptionalColorAnimation,
    PageStorage, PageStorageBucket, PageStorageIdentifier, PageStorageKey, PopAttempt, PopScope,
    PopScopeController, RootRestorationScope, UnmanagedRestorationScope,
    current_page_storage_bucket, current_restoration_scope,
};
pub use navigation_scopes::OverlayPortal;
pub use notification::NotificationDispatcher;
pub use overscroll_indicator::{
    GlowingOverscrollIndicator, StretchEffect, StretchingOverscrollIndicator,
};
pub use painting_effects::{
    AlignedShapeGradient, BeveledRectangleBorder, BlurStyle, Border, BorderComposition, BorderDirectional, BorderRadius,
    BorderRadiusDirectional, BorderRadiusGeometry, BorderRadiusResolutionError, BorderSide, BorderStyle, BoxBorder, BoxBorderLayer, BoxBorderStack,
    BoxBorderValue, BoxDecoration, BoxLinearGradient, BoxRadialGradient, BoxShadow, BoxShape, BoxSweepGradient,
    CircleBorder, CircularNotchedRectangle, ClipRSuperellipse, CompoundBorder, ContinuousRectangleBorder, CustomBoxBorder, CustomBoxBorderHandle, CustomPainter,
    CustomPainterSemantics, DecoratedShapeBox, DecorationImage, DecorationImageErrorHandler, DecorationImageSource, DecorationPosition, GridPaper,
    EllipticalRoundedRect, ImageFiltered, ImageIcon, LinearBorder, LinearBorderEdge, NotchedShape, OvalBorder,
    PhysicalModel, PhysicalShape, Radius, RawImage, RoundedCircleOutline, RoundedRectangleBorder, RoundedSuperellipseBorder,
    SemanticsBuilder, ShapeBorder, ShapeBorderClipper, ShapeDecoration, SnapshotWidget,
    StadiumBorder, StarBorder, StarBorderError, StarBorderUpdate, TableBorder, TileMode,
    lerp_shape_border,
};
pub use platform_view::{
    PlatformViewController, PlatformViewHitTestBehavior, PlatformViewPlacement, PlatformViewSurface,
};
pub use platform_widgets::{
    MenuDispatchResult, MenuItemId, MenuOwnerId, NoopPlatformMenuDelegate, PlatformMenu,
    PlatformMenuBar, PlatformMenuBarController, PlatformMenuBuildError, PlatformMenuDelegate,
    PlatformMenuEntry, PlatformMenuEvent, PlatformMenuItem, PlatformMenuItemGroup,
    PlatformMenuShortcut, PlatformMenuSnapshot, PlatformMenuSnapshotNode, PlatformMenuUpdate,
    ShortcutModifiers,
};
pub use provider_image::{
    ImageErrorBuilder, ImageFrameBuilder, ImageLoadingBuilder, ProviderImage,
};
pub use radio_selection::{RadioGroup, RawRadio, SelectableRegion};
pub use raw_input::{
    ErasedGestureRecognizerFactory, GestureRecognizer, GestureRecognizerFactory,
    GestureRecognizerFactoryError, GestureRecognizerFactoryWithHandlers, Listener,
    ListenerCallbacks, MetaData, MouseCursor, MouseRegion, MouseRegionCallbacks, PointerDeviceKind,
    PointerSignalResolver, RawGestureDetector, RawPointerEvent, TapRegion, TapRegionCallbacks,
    TapRegionGroupId, TapRegionSurface, TextFieldTapRegion,
};
pub use raw_menu_anchor::{
    DismissIntent, DismissMenuAction, MenuCloseRequested, MenuController, MenuOpenRequested, RawMenuAnchor,
    RawMenuAnchorChildBuilder, RawMenuAnchorGroup, RawMenuAnchorOverlayBuilder, RawMenuOverlayInfo,
};
pub use raw_tooltip::{
    RawTooltip, RawTooltipBuilder, RawTooltipController, RawTooltipDurations,
    RawTooltipTriggerMode, RawTooltipVisibility, TooltipAnimationBuilder, TooltipComponentBuilder,
    TooltipDurations,
    TooltipPositionContext, TooltipPositionDelegate, TooltipTriggerMode, TooltipVisibility,
};
pub use repeating_animation_builder::{
    RepeatMode, RepeatingAnimationBuilder, RepeatingAnimationWidgetBuilder,
};
pub use safe_area::{RemovePadding, SafeArea};
pub use scroll_aware_image_provider::{ScrollAwareContext, ScrollAwareImageProvider};
pub use scroll_behavior::{
    AndroidOverscrollIndicator, MultitouchDragStrategy, ScrollBehavior, TOUCH_LIKE_DEVICE_KINDS,
};
pub use scrolling::{
    CustomScrollView, DecoratedSliver, EnsureVisible, FloatingHeaderSnapMode, GridView, ListBody,
    ListView, NestedScrollView, NotificationListener, PageController, PageView,
    PersistentHeaderBuilder, PinnedHeaderSliver, ReorderableDelayedDragStartListener,
    ReorderableDragStartListener, ReorderableList, ScrollCacheExtent, ScrollNotificationObserver,
    RevealedOffset, ScrollIncrementCalculator, ScrollIncrementDetails, ScrollIncrementType,
    ScrollIntent, ScrollPositionAlignmentPolicy, Scrollable, SemanticIndexCallback,
    ShowOnScreen, ShrinkWrappingViewport,
    SingleChildScrollView, Sliver, SliverAnimatedList, SliverAnimatedListController,
    SliverAnimatedOpacity, SliverChildBuilderDelegate, SliverChildDelegate,
    SliverChildListDelegate, SliverConstrainedCrossAxis, SliverCrossAxisExpanded,
    SliverCrossAxisGroup, SliverEnsureSemantics, SliverFadeTransition, SliverFillRemaining,
    SliverFillViewport, SliverFixedExtentList, SliverFloatingHeader, SliverGrid,
    PersistentHeaderShowOnScreenConfiguration,
    SliverGridDelegate, SliverHeaderOverscrollBehavior, SliverHeaderScrollBehavior,
    SliverIgnorePointer, SliverLayout, SliverLayoutBuilder, SliverList, SliverMainAxisGroup,
    SliverNaturalHeader, SliverOffstage, SliverOpacity, SliverOverlapAbsorber, SliverOverlapHandle,
    SliverOverlapInjector, SliverPadding, SliverPaintOrder, SliverPersistentHeader,
    SliverPrototypeExtentList, SliverReorderController, SliverReorderableList,
    SliverResizingHeader, SliverSafeArea, SliverSemantics, SliverToBoxAdapter,
    SliverVariedExtentList, SliverVisibility, SliverVisibilityFlagError, Viewport,
    SliverBoxPaintConfiguration, SliverBoxPainter, SliverBoxPainterFactory,
};
pub use selection::{
    SelectableChildPolicy, SelectedContent, SelectedContentRange, SelectionAreaController,
    SelectionContainerDelegate, SelectionDetails, SelectionGeometry, SelectionHandleType,
    SelectionListenerNotifier, SelectionPoint, SelectionStatus,
};
pub use selection_container::SelectionContainer;
pub use selection_listener::SelectionListener;
pub use semantics::{BlockSemantics, ExcludeSemantics, MergeSemantics, Semantics};
pub use semantics_debugger::{DEFAULT_SEMANTICS_DEBUGGER_NODE_LIMIT, SemanticsDebugger};
pub use shortcut_registry::{
    CharacterActivator, RegistryActivator, ShortcutRegistrar, ShortcutRegistry,
    ShortcutRegistryEntry,
};
pub use size_changed::{SizeChangedLayoutNotification, SizeChangedLayoutNotifier};
pub use spell_check::{
    DefaultSpellCheckProvider, SpellCheckConfiguration, build_text_span_with_spell_check_suggestions,
};
pub use stateful_builder::{StateSetter, StatefulBuilder};
pub use text_selection_handles::{
    SELECTION_HANDLE_SIZE, TextSelectionHandle, TextSelectionHandles, TextSelectionHandlesBuilder,
    TextSelectionHandlesState,
};
pub use transient::{
    OverlayChildLayoutInfo, TransientAlignment, TransientDismissPolicy, TransientDismissReason,
    TransientPlacement, TransientPlacementInput, TransientPlacementMode, TransientPlacementResult,
    TransientSide, TransientSurfaceId, TransientSurfaceSnapshot, place_transient,
};
pub use tree::{TextEditingCallback, TextEditingCallbackError};
pub use tree::{
    AffineController, AffineParts, BoxFit, BuildContext, ColorFiltered, DecoratedBox, EditableText,
    FadeTransition, FlexOverflowDiagnostic, GlobalKey, Icon, Image, ImageRepeat, Opacity,
    OpacityController, PathView,
    RichParagraph, RotationController, RotationTransition, ScaleController, ScaleTransition,
    SlideTransition, Text, TextFieldInputSnapshot, TextInputActionHint, TextInputTypeHint,
    ToolbarOptions, Transform, TranslationController, Widget,
};
pub use tweens::{
    AlignmentDirectionalTween, AlignmentTween, BorderRadiusTween, BorderTween, BoxConstraintsTween,
    DecorationTween, EdgeInsetsDirectionalTween, EdgeInsetsTween, LerpTween,
    OptionalBorderRadiusTween, OptionalBorderTween, TextStyleTween,
};
pub use utilities::{
    Banner, BannerLocation, CheckedModeBanner, ErrorWidget, Expansible, PerformanceOverlay,
};

// Internal implementation aliases.  These names are deliberately
// `pub(crate)`: sibling crates use the documented `internal` bridge instead
// of observing retained-tree implementation types as Widgets API.
pub(crate) use gestures::GestureCallbacks;
pub(crate) use tree::{Blur, BlurController, DropShadow, ExplicitSemantics, WidgetKind};
