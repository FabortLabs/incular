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
mod compositing;
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
mod environment;
pub mod extensions;
mod external_drop;
mod fade_in_image;
mod flutter_logo;
mod focus_keyboard;
mod forms;
mod gestures;
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
mod overscroll_indicator;
mod painting_effects;
mod platform_widgets;
mod radio_selection;
mod raw_input;
mod raw_menu_anchor;
mod raw_tooltip;
mod recursion;
mod render_object;
mod repeating_animation_builder;
mod safe_area;
mod scroll_behavior;
mod scrolling;
mod selection;
mod selection_container;
mod selection_listener;
mod semantics;
mod semantics_debugger;
mod size_changed;
mod transient;
mod tree;
mod util;
mod utilities;

// Re-export only the Flutter Widgets vocabulary.  Retained-tree IDs,
// renderer diagnostics, and Incular-only helpers stay private to this crate;
// the facade's `internal` module is the sole explicitly documented bridge for
// sibling implementation crates.
pub use advanced_scrolling::{
    CacheExtentStyle, ChangeReportingBehavior, ChildVicinity, DiagonalDragBehavior,
    DraggableAnimationStep, DraggableNotificationSubscription, DraggableScrollableActuator,
    DraggableScrollableController, DraggableScrollableNotification, DraggableScrollableSheet,
    DraggableScrollableState, DraggableSheetDelta, DraggableSheetExtent, DraggableSizeAnimation,
    DraggableSnap, DraggableSnapTarget, FixedExtentScrollController, ListWheelScrollView,
    ListWheelViewport, RawScrollbar, RawScrollbarGeometry, RawScrollbarOrientation,
    RawScrollbarStyle, TwoDimensionalChildDelegate, TwoDimensionalChildLayout,
    TwoDimensionalConstraints, TwoDimensionalScrollDelta, TwoDimensionalScrollView,
    TwoDimensionalScrollable, TwoDimensionalViewport, TwoDimensionalViewportLayout,
    WheelChildDelegate, WheelChildLayout, WheelLayout, WheelMatrix, WheelProjection,
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
    AnimatedSwitcher, AnimatedSwitcherLayoutBuilder, AnimatedSwitcherTransitionBuilder,
};
pub use animation_transitions::{
    AlignTransition, DecoratedBoxTransition, DefaultTextStyleTransition, DualTransitionBuilder,
    MatrixTransition, PositionedTransition, RelativePositionedTransition, SizeTransition,
};
pub use app_shell::{
    ApplicationBootstrapHost, ApplicationBootstrapOptions, ApplicationBootstrapSpec,
    AuxiliaryViewError, AuxiliaryViewHandle, AuxiliaryViewHost, AuxiliaryViewOutcome,
    AuxiliaryViewRequest, BackButtonDispatcher, BackCallbackSubscription, BasicRouterDelegate,
    ClosureRouteInformationParser, MemoryRouteInformationProvider, NavigationNotification,
    NavigationNotificationKind, NavigationNotificationListener, NavigationNotificationSubscription,
    NoopAuxiliaryViewHost, NoopWindowChromeSink, RootBackButtonDispatcher, RouteInformation,
    RouteInformationListener, RouteInformationParser, RouteInformationProvider,
    RouteInformationReportingType, RouteInformationSubscription, Router, RouterConfig, RouterData,
    RouterDelegate, RouterDelegateListener, RouterDelegateSubscription, RouterError,
    StringRouteInformationParser, Title, TitleController, TitleData, TitleError, View, ViewAnchor,
    ViewAnchorController, ViewAnchorData, ViewAnchorSubscription, ViewController, ViewData,
    ViewEvent, ViewId, ViewLifecycle, ViewMetrics, ViewSubscription, WidgetsApp,
    WidgetsAppController, WidgetsAppData, WindowChromeSink, WindowDragRegion, WindowResizeRegion,
    normalize_route_location,
};
pub use asset_bundle::DefaultAssetBundle;
pub use async_builders::{
    AsyncSnapshot, AsyncWidgetBuilder, ConnectionState, FutureBuilder, StreamBuilder,
    StreamBuilderBase, StreamEvent, UiCompleter, UiFuture, UiStream, UiStreamController,
    UiStreamSubscription,
};
pub use compositing::{
    AnnotatedRegion, BackdropFilter, BackdropGroup, CompositedTransformFollower,
    CompositedTransformTarget, ShaderCallback, ShaderMask,
};
pub use custom_layout::{
    CustomMultiChildLayout, CustomSingleChildLayout, Flow, FlowDelegate, FlowPaintingContext,
    IgnoreBaseline, LayoutChildId, LayoutId, MultiChildLayoutContext, MultiChildLayoutDelegate,
    PositionedDirectional, SingleChildLayoutDelegate,
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
    Orientation, OrientationBuilder, PrimaryScrollController, ScrollConfiguration,
    SensitiveContent, SensitiveContentHost, TickerMode,
};
pub use fade_in_image::FadeInImage;
pub use flutter_logo::FlutterLogo;
pub use focus_keyboard::{
    Action, ActionInvocationPhase, ActionListener, ActionResult, Actions, CallbackShortcuts,
    Command, CommandId, ExcludeFocus, ExcludeFocusTraversal, Focus, FocusManager, FocusNode,
    FocusScope, FocusScopeNode, FocusScopeSubscription, FocusTraversalGroup, FocusTraversalOrder,
    FocusTraversalPolicy, FocusTraversalPolicyKind, FocusableActionDetector, Intent,
    KeyboardListener, LogicalShortcutKey, OrderedTraversalPolicy, ReadingOrderTraversalPolicy,
    ShortcutActivator, ShortcutKey, ShortcutTrigger, Shortcuts, SingleActivator,
    WidgetOrderTraversalPolicy,
};
pub use forms::{
    AutocompleteHighlightedOption, AutofillGroup, AutovalidateMode, FilteringTextInputFormatter,
    Form, FormController, FormField, FormFieldState, FormState, LengthLimitingTextInputFormatter,
    MaxLengthEnforcement, RawAutocomplete, TextInputFormatter, UndoHistory,
};
pub use gestures::{
    AbsorbPointer, DragEndDetails, DragStartDetails, DragUpdateDetails, GestureDetector,
    HitTestBehavior, IgnorePointer, TapUpDetails,
};
pub use implicit_animations::{
    AnimatedAlign, AnimatedContainer, AnimatedDefaultTextStyle, AnimatedFractionallySizedBox,
    AnimatedOpacity, AnimatedPadding, AnimatedPhysicalModel, AnimatedPositioned,
    AnimatedPositionedDirectional, AnimatedRotation, AnimatedScale, AnimatedSlide,
    TweenAnimationBuilder, TweenAnimationWidgetBuilder,
};
pub use incular_animation::{
    Animatable, Animation, AnimationController, AnimationStatus, Curve, Curves, Tween,
};
pub use incular_config::{
    Alignment, AlignmentDirectional, Axis, AxisDirection, Brightness, Clip, CrossAxisAlignment,
    FlexFit, FractionalOffset, MainAxisAlignment, MainAxisSize, StackFit, TextDirection,
    TransientPresentation, TransientRole, VerticalDirection, WrapAlignment, WrapCrossAlignment,
};
pub use incular_core::{Color, Key, LocalKey, Offset, Rect, Size, UniqueKey, ValueKey};
pub use incular_image::{AssetImage, FileImage, ImageConfiguration, ImageProvider, MemoryImage};
pub use incular_rendering::{
    Annotation, BlendMode, Canvas, ColorFilter, FilterQuality, LayerAnchor, LayerLink,
    LinearGradient, Paint, PaintStyle, Path, RRect, RadialGradient, Shader, Shadow, SweepGradient,
};
pub use incular_scroll::{
    ClampingScrollPhysics, ScrollController, ScrollMetrics, ScrollNotification,
    ScrollNotificationSubscription, ScrollNotificationType, ScrollPhysics,
    ScrollViewKeyboardDismissBehavior, SliverConstraints,
};
pub use incular_text::{
    FontFeature, FontVariation, FontWeight, IconData, RichText, StrutStyle, TextAlign,
    TextBaseline, TextEditingController, TextEditingValue, TextHeightBehavior, TextOverflow,
    TextRange, TextScaler, TextSelection, TextSpan, TextStyle, TextWidthBasis, WidgetSpan,
};
pub use indexed_semantics::IndexedSemantics;
pub use inherited_data::{
    InheritedModel, InheritedModelData, SharedAppData, SharedAppDataKey, SharedAppDataStore,
};
pub use interactive_viewer::{InteractiveViewer, PanAxis, TransformationController};
pub use layout::{
    Align, AspectRatio, Baseline, Builder, Center, ClipOval, ClipPath, ClipRRect, ClipRect,
    ColoredBox, Column, ConstrainedBox, ConstraintsTransformBox, Container, CustomPaint, Expanded,
    FittedBox, Flex, Flexible, FractionalTranslation, FractionallySizedBox, IndexedStack,
    IntrinsicHeight, IntrinsicWidth, KeyedSubtree, LayoutBuilder, LimitedBox, NavigationToolbar,
    Offstage, OverflowBar, OverflowBox, Padding, Positioned, PreferredSize, RepaintBoundary,
    RotatedBox, Row, SizedBox, SizedOverflowBox, Spacer, Stack, Table, TableCell, UnconstrainedBox,
    Visibility, Wrap,
};
pub use magnifier::{
    MagnifierBuilder, MagnifierController, MagnifierDecoration, MagnifierInfo, RawMagnifier,
    TextMagnifierConfiguration,
};
/// The navigation-scope dispatcher is kept under an explicit name so a future
/// app-shell `BackButtonDispatcher` can coexist in this facade without a
/// duplicate public binding.
pub use navigation::BackButtonDispatcher as NavigationBackButtonDispatcher;
pub use navigation::{
    AnimatedModalBarrier, BackButtonListener, BackDispatchReport, BackHandlerResult,
    BackRegistration, NavigatorPopHandler, NavigatorPopHandlerController, PageStorage,
    PageStorageBucket, PageStorageIdentifier, PageStorageKey, PopAttempt, PopScope,
    PopScopeController, RootRestorationScope, UnmanagedRestorationScope,
    current_page_storage_bucket, current_restoration_scope,
};
pub use navigation_scopes::OverlayPortal;
pub use overscroll_indicator::{
    GlowingOverscrollIndicator, StretchEffect, StretchingOverscrollIndicator,
};
pub use painting_effects::{
    Border, BorderDirectional, BorderRadius, BorderRadiusDirectional, BorderSide, BorderStyle,
    BoxBorder, BoxDecoration, BoxShadow, BoxShape, ClipRSuperellipse, CustomPainter,
    DecoratedShapeBox, DecorationImage, GridPaper, ImageFiltered, ImageIcon, Radius, RawImage,
    RoundedRectangleBorder, RoundedSuperellipseBorder, ShapeBorder, ShapeDecoration,
    SnapshotWidget, StadiumBorder, StarBorder, TileMode,
};
pub use platform_widgets::{
    MenuDispatchResult, MenuItemId, MenuOwnerId, NoopPlatformMenuDelegate, PlatformMenu,
    PlatformMenuBar, PlatformMenuBarController, PlatformMenuBuildError, PlatformMenuDelegate,
    PlatformMenuEntry, PlatformMenuEvent, PlatformMenuItem, PlatformMenuItemGroup,
    PlatformMenuShortcut, PlatformMenuSnapshot, PlatformMenuSnapshotNode, PlatformMenuUpdate,
    ShortcutModifiers,
};
pub use radio_selection::{RadioGroup, RawRadio, SelectableRegion};
pub use raw_input::{
    ErasedGestureRecognizerFactory, GestureRecognizer, GestureRecognizerFactory,
    GestureRecognizerFactoryError, GestureRecognizerFactoryWithHandlers, Listener,
    ListenerCallbacks, MetaData, MouseCursor, MouseRegion, MouseRegionCallbacks, PointerDeviceKind,
    RawGestureDetector, RawPointerEvent, TapRegion, TapRegionCallbacks, TapRegionGroupId,
    TapRegionSurface, TextFieldTapRegion,
};
pub use raw_menu_anchor::{
    MenuCloseRequested, MenuController, MenuOpenRequested, RawMenuAnchor,
    RawMenuAnchorChildBuilder, RawMenuAnchorGroup, RawMenuAnchorOverlayBuilder, RawMenuOverlayInfo,
};
pub use raw_tooltip::{
    RawTooltip, RawTooltipBuilder, RawTooltipController, RawTooltipDurations,
    RawTooltipTriggerMode, RawTooltipVisibility, TooltipComponentBuilder, TooltipDurations,
    TooltipTriggerMode, TooltipVisibility,
};
pub use repeating_animation_builder::{
    RepeatMode, RepeatingAnimationBuilder, RepeatingAnimationWidgetBuilder,
};
pub use safe_area::SafeArea;
pub(crate) use safe_area::SafeAreaPolicy;
pub use scroll_behavior::{
    AndroidOverscrollIndicator, MultitouchDragStrategy, ScrollBehavior, TOUCH_LIKE_DEVICE_KINDS,
};
pub use scrolling::{
    CustomScrollView, DecoratedSliver, GridView, ListBody, ListView, NestedScrollView,
    NotificationListener, PageController, PageView, PinnedHeaderSliver,
    ReorderableDelayedDragStartListener, ReorderableDragStartListener, ReorderableList,
    ScrollNotificationObserver, Scrollable, SemanticIndexCallback, SingleChildScrollView, Sliver,
    SliverAnimatedList, SliverAnimatedListController, SliverAnimatedOpacity,
    SliverChildBuilderDelegate, SliverChildDelegate, SliverChildListDelegate,
    SliverConstrainedCrossAxis, SliverCrossAxisExpanded, SliverCrossAxisGroup,
    SliverEnsureSemantics, SliverFadeTransition, SliverFillRemaining, SliverFillViewport,
    SliverFixedExtentList, SliverFloatingHeader, SliverGrid, SliverGridDelegate,
    SliverHeaderOverscrollBehavior, SliverHeaderScrollBehavior, SliverIgnorePointer, SliverLayout,
    SliverLayoutBuilder, SliverList, SliverMainAxisGroup, SliverNaturalHeader, SliverOffstage,
    SliverOpacity, SliverOverlapAbsorber, SliverOverlapHandle, SliverOverlapInjector,
    SliverPadding, SliverPersistentHeader, SliverPrototypeExtentList, SliverReorderController,
    SliverReorderableList, SliverResizingHeader, SliverSafeArea, SliverSemantics,
    SliverToBoxAdapter, SliverVariedExtentList, SliverVisibility, Viewport,
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
pub use size_changed::SizeChangedLayoutNotifier;
pub use transient::{
    OverlayChildLayoutInfo, TransientAlignment, TransientDismissPolicy, TransientDismissReason,
    TransientPlacement, TransientPlacementInput, TransientPlacementMode, TransientPlacementResult,
    TransientSide, TransientSurfaceId, TransientSurfaceSnapshot, place_transient,
};
pub use tree::{
    BoxFit, BuildContext, ColorFiltered, DecoratedBox, EditableText, FadeTransition, Icon, Image,
    ImageRepeat, Opacity, RotationTransition, ScaleTransition, SlideTransition, Text,
    TextFieldInputSnapshot, TextInputActionHint, TextInputTypeHint, ToolbarOptions, Transform,
    Widget,
};
pub use utilities::{
    Banner, BannerLocation, CheckedModeBanner, ErrorWidget, Expansible, PerformanceOverlay,
};

// Internal implementation aliases.  These names are deliberately
// `pub(crate)`: sibling crates use the documented `internal` bridge instead
// of observing retained-tree implementation types as Widgets API.
pub(crate) use gestures::GestureCallbacks;
pub(crate) use tree::{Blur, BlurController, DropShadow, ExplicitSemantics, WidgetKind};
