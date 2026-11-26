#![doc = include_str!("../README.md")]

pub use incular_accessibility as accessibility;
pub use incular_animation as animation;
pub use incular_assets as assets;
pub use incular_config as config;
#[cfg(feature = "controls")]
pub use incular_controls as controls;
pub use incular_core as core;
pub use incular_core::licenses;
/// Clocks that work on every target; identical to `std::time` natively.
pub use incular_core::time;
pub use incular_gestures as gestures;
pub use incular_image as image;
pub use incular_layout as layout;
#[cfg(feature = "material")]
pub use incular_material as material;
pub use incular_navigation as navigation;
pub use incular_rendering as painting;
pub use incular_rendering as rendering;
pub use incular_runtime as runtime;
pub use incular_semantics as semantics;
pub use incular_text as text;
pub use incular_widgets as widgets;
pub use incular_widgets::widget_previews;

/// Public scrolling policies and notifications.
///
/// Retained extent indexes, scrollbar geometry, and viewport configuration
/// are implementation details of the widget/runtime boundary and are kept
/// out of the application facade.
pub mod scroll {
    pub use incular_widgets::{
        PersistentHeaderShowOnScreenConfiguration, RevealedOffset, ScrollIncrementCalculator,
        ScrollIncrementDetails, ScrollIncrementType, ScrollIntent,
        ShowOnScreen,
    };
    pub use incular_scroll::{
        BoundaryPhysics, ClampingScrollPhysics, DragStartBehavior, PageMetrics, PageMetricsUpdate,
        ScrollAnimation, ScrollAnimationCompletion,
        ScrollController, ScrollMetrics, ScrollNotification, ScrollNotificationSubscription,
        ScrollNotificationType, ScrollPhysics, ScrollViewKeyboardDismissBehavior, Scrollability,
        SnapPhysics,
    };
}

/// Reactive values and owner-mounted asynchronous primitives. This namespace
/// keeps [`reactive::Effect`] and [`reactive::Action`] distinct from Incular's rendering and
/// widget types with the same conventional names.
pub mod reactive {
    pub use incular_runtime::{
        Action, ActionDispatchError, ActionError, ActionState, Effect, Memo,
    };
}

/// Application-facing DevTools logging. Records appear in the DevTools
/// Console when the `devtools` feature is enabled and are dropped otherwise.
pub mod devtools {
    pub use incular_runtime::{LOG_MESSAGE_LIMIT, LOG_QUEUE_CAPACITY, LogLevel, log};
}

/// High-level application testing tools. These drive Incular's normalized
/// input/event path in-process and capture rendered application frames without
/// injecting OS input or capturing the desktop.
pub mod testing {
    pub use incular_runtime::{PointSpace, Screenshot, Simulation, SimulationError};
}

/// Motion: `.motion()` states, presence, layout and shared elements; `.transition()`;
/// `.animate()` chains; drag, drop and reorder; `with_transition` transactions and
/// `view_transition`. The prelude brings the extension traits and the common functions;
/// the vocabulary types (`Pose`, `Presence`, `Move`, ..) live here.
pub mod motion {
    pub use incular_widgets::motion::*;
    pub use incular_widgets::{TransitionExt, Transitionable};
}

/// Common application-facing types for the initial native UI slice.
pub mod prelude {
    // Motion: the extension traits and the functions every app reaches for.
    pub use incular_animation::{
        AnimationController, AnimationStatus, BoundedFrictionSimulation, ClampedSimulation, Curve,
        CurveBlend, CurveChain, Curves, FrictionSimulation, GravitySimulation, ScrollSpringSimulation,
        Simulation, SpringDescription, SpringSimulation, SpringType, Tolerance, Tween,
        TweenSegment, TweenSequence, TweenValue,
    };
    pub use incular_widgets::TransitionExt;
    pub use incular_widgets::motion::{
        DndExt, MotionExt, ReorderExt, Spring, Transition, ViewTransitionExt, ms, view_transition,
        with_transition, without_transition,
    };
    // Flutter's animation graph, tweens and frame scheduler.
    pub use incular_animation::{
        AlwaysStoppedAnimation, AnimatableExt, AnimationSource, AnimationStyle, CompoundAnimation,
        ConstantTween, CurveTween, CurvedAnimation, DoubleAnimation, DrivenAnimation,
        FrameCallbackId, ProxyAnimation, RepeatOptions, ReverseAnimation, ReverseTween, StepTween,
        Ticker, TrainHoppingAnimation, add_persistent_frame_callback, add_post_frame_callback,
        always_complete, always_dismissed, animation_max, animation_mean, animation_min,
        cancel_frame_callback, schedule_frame, schedule_frame_callback,
        schedule_frame_callback_with_request,
    };
    // Flutter's named tweens.
    pub use incular_animation::{ColorTween, IntTween, Matrix4Tween, RectTween, SizeTween};
    // Route observation, focus order, application-wide shortcuts, inherited
    // notifiers, text context menus and typed restorable values.
    pub use incular_config::{
        Alignment, AlignmentDirectional, ApplicationDefaults, Axis, AxisDirection, Brightness,
        Constraints, ContentSensitivity, CrossAxisAlignment, DisplayFeature, DisplayFeatureState,
        DisplayFeatureType, EdgeInsets, EdgeInsetsDirectional, FlexFit, FractionalOffset,
        InputCapabilities, Locale, LocaleResolver, LocalizationCatalog, LocalizationError,
        LocalizedMessage, MainAxisAlignment, MainAxisSize, PluralCategory, PluralForms,
        RuntimeEnvironment, TextDirection, TransientPresentation, TransientRole, TransparencyMode,
        VerticalDirection, ViewConstraints, WidgetDefaults, WindowSizePolicy, WrapAlignment,
        WrapCrossAlignment,
    };
    pub use incular_core::{
        BACK_POINTER_BUTTON, BitField, ChangeImpact, ChangeNotifier, ChangeNotifierSubscription, Code, Color,
        DebugPrintBuffer, DirtyFlags, FORWARD_POINTER_BUTTON, HslColor, HsvColor, ImeEvent, InputEvent,
        Invalidation, Key, KeyState, KeyboardEvent, KeyboardKey, Lerp, LocalKey, Location,
        Modifiers, NamedKey,
        NormalizedPressure, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
        PointerSampleMetadata, Rect, RestorationKey, RestorationKeyError, RestorationScope,
        SECONDARY_POINTER_BUTTON, Size, StylusMetadata, StylusOrientation, TERTIARY_POINTER_BUTTON,
        TrackpadGesture, TrackpadGestureKind, TrackpadGesturePhase, Transform as AffineTransform,
        UniqueKey, ValueKey, ValueNotifier, WindowResizeDirection, additional_pointer_button_mask,
        debug_double_precision, debug_format_double, debug_print_synchronously, debug_word_wrap,
        set_debug_double_precision,
    };
    pub use incular_gestures::{
        FocusExtent, FocusRect, PointerRouteError, PointerRouteErrorHandler, PointerRouteId, PointerRouter,
        RoutedPointerEvent, WeakPointerRouter,
    };
    pub use incular_image::{
        AnyImageProvider, AssetBundleImageKey, AssetImage, DecodedImage, ExactAssetImage, FileImage,
        ImageAssetBundle, ImageCache, ImageCacheDiagnostics, ImageChunkEvent, ImageConfiguration,
        ImageError, ImageHandle, ImageId, ImageLoadState, ImageProvider, ImageResizeOptions,
        ImageSource, ImageStream, ImageStreamCompleter, MemoryImage, ResizeImage, ResizeImageKey, ResizeImagePolicy,
    };
    pub use incular_navigation::{
        BackDispatchReport, BackDispatcher, BottomSheet, Dialog, HeroController, ModalBarrier,
        NAVIGATOR_SNAPSHOT_FORMAT_VERSION, NavigationEvent, NavigationRestoreReport, Navigator,
        NavigatorObserver, NavigatorSnapshot, Overlay, OverlayEntry, Page, PageRouteBuilder,
        PopDecision, PopResult, RestorableNavigationError, RestorableRoute,
        RestorableRouteBuildError, RestorableRouteId, RestorableRouteIdError,
        RestorableRouteRegistrationError, Route, RouteLifetime, RouteLifetimeSubscription,
        RoutePresentation, RouteRegistry, RouteResult, RouteScopeKey, RouteScopeKeyError,
        RouteSettings, RouteTransition,
    };
    pub use incular_navigation::{ModalRoute, RouteAware, RouteAwareCallbacks, RouteObserver};
    #[cfg(any(
        feature = "desktop-host",
        feature = "web-host",
        feature = "mobile-host"
    ))]
    pub use incular_platform::{
        AdvancedInputCapabilities, ApplicationActivation, ApplicationBadge,
        ApplicationServiceCapabilities, ApplicationShellError, ApplicationShellFeature,
        CapabilitySupport, ClipboardCapabilities, ClipboardError, ClipboardWriteReport,
        ContentSensitivityBackend, ContentSensitivityCapability, ContentSensitivityNoOpReason,
        ContentSensitivityOutcome, CursorGrabMode, DataTransfer, DataTransferCapabilities,
        DisplayId, DisplayPlacementArea, DisplayPlacementCapabilities, DisplaySnapshot,
        DocumentActivation, DocumentDescriptor, ExternalDragEvent, ExternalDragPhase,
        ExternalDragResponse, FileContentType, FileContentTypeKind, FileDialogCapabilities,
        FileDialogError, FileDialogFilter, FileDialogFilterId, FileDialogKind, FileDialogOption,
        FileDialogOptions, FileDialogValidationError, FileExtension, Fullscreen,
        GlobalShortcutChord, GlobalShortcutChordError, GlobalShortcutError, GlobalShortcutId,
        LaunchActivation, LogicalDisplayPosition, LogicalScreenPosition, LogicalScreenRect,
        LogicalSizeLimits, LogicalSizeLimitsError, LogicalWindowPosition,
        LogicalWindowPositionError, MediaType, MemoryContentSensitivityBackend,
        MemoryTextInputAdapter, NativeMenuCapabilities, NativeOperationCompletion, NativeRequestId,
        NoopContentSensitivityBackend, NotificationAction, NotificationActionId, NotificationId,
        NotificationPresentation, PhysicalDisplayPosition, PhysicalScreenPosition,
        PhysicalScreenRect, PlatformCapabilities, PlatformOperationError,
        PlatformOperationErrorKind, PlatformOperationResult, PointerMetadata, SingleInstancePolicy,
        SingleInstancePolicyError, TaskbarDockState, TaskbarProgress, TaskbarProgressError,
        TextCapitalization, TextInputAction, TextInputAdapter, TextInputClientId, TextInputCommand,
        TextInputConfiguration, TextInputState, TextInputTraits, TextInputType, TransferData,
        TransferDataError, TransferFormat, TransferFormatCapabilities, TransferImage, TransferItem,
        TransferOperation, TransferOperations, TransferReadRequest, TransferRepresentation,
        TransientSurfaceCapabilities, TrayItemId, TrayItemPresentation, UrlActivation,
        UserAttentionType, WindowCommand, WindowControlCapabilities, WindowEvent, WindowEventKind,
        WindowIcon, WindowIconError, WindowId, WindowLevel, WindowLifecycle, WindowObservedState,
        WindowOperation, WindowOptions, WindowOptionsError, WindowRequestedState,
    };
    pub use incular_rendering::{
        BackdropKey, BlendMode, Border as RenderBorder, Brush, Canvas, ColorFilter, ColorMatrix,
        CornerRadii, Decoration, DisplayList, DropShadowEffect, Effect, EffectChain, FillRule,
        FilterQuality, GaussianBlur, GradientId, GradientStop, GradientStops, GradientStopsError,
        ImageSampling, LineCap, LineJoin, LinearGradient, Paint, PaintStyle, Path, PathBuilder,
        PathId, RRect, RadialGradient, Shader, Shadow, Stroke, SweepGradient, blend_premultiplied,
    };
    /// `localStorage` persistence, the browser default for restoration.
    #[cfg(target_arch = "wasm32")]
    pub use incular_runtime::BrowserRestorationStore;
    pub use incular_runtime::FocusKeySubscription;
    /// Browser targets have no Tokio runtime.
    #[cfg(not(target_arch = "wasm32"))]
    pub use incular_runtime::TokioHandle;
    pub use incular_runtime::{
        AccessibilityDiagnostics, Application, ApplicationDiagnostics, ApplicationLifecycle,
        AsyncState, AsyncValue, BudgetStatistics, BuildContext, ClaimedRestorationScope,
        EditingDiagnostics, FileRestorationStore, FocusDiagnostics, FrameHistory, FrameRecord,
        FrameStatistics, GpuSample, HardwareKeyHandlerError, InMemoryRestorationStore,
        LastWindowPolicy, Memo,
        NativeOperationCompletionStatus, NativeOperationRequest, NativeWindowCommand,
        OutletAttachError, OutletError, OutletId, OutletPlacement, PendingRestorationScope, PerformanceHub,
        PerformanceSnapshot, ProfilerMode, RenderFrameMetrics, Restorable, RestorableWindowFactory,
        RestorationConfig, RestorationDiagnostics, RestorationHandle, RestorationHandoffError, RestorationMigration,
        RestorationStore, RestorationStoreError, RouteFocusOwner, RouteFocusState, RouteOutlet,
        RouteTaskBinding, Runtime, RuntimeDiagnostics, RuntimeErrorReport, SchedulerCounters,
        Signal, Task, TaskFailure, TaskHandle, TaskScope, UiDispatcher, UndoHistoryController,
        UndoHistoryState, WindowCommandEnqueueError, WindowDiagnostics, WindowError, WindowHandle,
        WindowOpener, WindowPlacementError, WindowRestorationId,
    };
    #[cfg(any(
        feature = "desktop-host",
        feature = "web-host",
        feature = "mobile-host"
    ))]
    pub use incular_runtime::{
        ActivationRouteBridge, ApplicationActivationService, ApplicationActivationSubscription,
        ApplicationShellService, FileDialogRequest, FileDialogService, GlobalShortcutRegistration,
        GlobalShortcutRegistrationRequest, GlobalShortcutService,
        GlobalShortcutUnregistrationRequest, NotificationHandle, TrayItemHandle,
    };
    pub use incular_runtime::{
        RestorableBool, RestorableBoolN, RestorableDouble, RestorableDoubleN, RestorableInt,
        RestorableIntN, RestorableString, RestorableStringN,
    };
    pub use incular_semantics::{
        AccessibilityFocusBlockType, Role as SemanticRole, SemanticAction, SemanticActionKind,
        SemanticNodeId, SemanticState, SemanticValidationResult, SemanticsDiagnostics,
        SemanticsInputType, SemanticsTree,
    };
    pub use incular_text::{
        EditableText as TextEditingModel, FontFamily, FontFeature, FontStyle, FontVariation,
        FontWeight, IconData, LineHeight, RichText, SelectionChangedCause, StrutStyle, TextAffinity,
        TextAlign,
        TextBaseline, TextCaretPosition, TextDecoration, TextDecorationStyle,
        TextEditingController, TextEditingValue, TextHeightBehavior, TextLayoutOptions,
        TextLeadingDistribution, TextMetricAdjustment, TextOverflow, TextRange, TextScaler,
        TextSelection, TextShadow, TextSpan, TextStyle, TextStyleApply, TextWidthBasis, WidgetSpan,
        lerp_font_variations,
    };
    pub use incular_config::MediaQueryEdges;
    pub use incular_widgets::{StateSetter, StatefulBuilder, SystemSound, SystemSoundType};
    pub use incular_widgets::{BoxRadialGradient, BoxSweepGradient};
    pub use incular_widgets::extensions::ExternalDropTarget;
    pub use incular_widgets::ModalBarrier as ModalBarrierWidget;
    pub use incular_widgets::{LayoutChildError, LayoutRelayout, OverflowBarAlignment, OverflowBoxFit};
    pub use incular_widgets::{
        BorderRadiusGeometry, EllipticalRoundedRect, FittedSizes, apply_box_fit,
        position_dependent_box,
    };
    pub use incular_widgets::{
        KeyEventResult, OptionalBorderRadiusTween, OptionalBorderTween, UnfocusDisposition,
    };
    pub use incular_widgets::{
        AlignmentDirectionalTween, AlignmentTween, BorderRadiusTween, BorderTween,
        BoxConstraintsTween, CapturedThemes, CharacterActivator, ContextMenuButtonItem,
        ContextMenuButtonType, DecorationTween, EdgeInsetsDirectionalTween, EdgeInsetsTween,
        EditableTextContextMenuBuilder, FocusOrder, InheritedNotifier, LerpTween, LogicalKeySet,
        RegistryActivator, ShortcutRegistrar, ShortcutRegistry, ShortcutRegistryEntry,
        TextContextMenu, TextSelectionToolbarAnchors, TextStyleTween,
    };
    // Developer-experience layers: chainable modifiers, effect chains and
    // responsive breakpoints.
    pub use incular_widgets::extensions::{
        Animate, AnimateExt, AnimateRepeat, Breakpoint, Condition, ResponsiveBreakpoints,
        ResponsiveBreakpointsData, ResponsiveRowColumn, ResponsiveScaledBox, ResponsiveValue,
        ResponsiveVisibility, WidgetExt,
    };
    // Explicit animation builders, the transition controllers and Flutter's
    // `RelativeRect`.
    pub use incular_widgets::{
        AbsorbPointer, Action, ActionInvocationPhase, ActionListener, ActionResult, Actions, Align,
        AlignTransition, AnimatedAlign, AnimatedContainer, AnimatedCrossFade,
        AnimatedDefaultTextStyle, AnimatedFractionallySizedBox, AnimatedGrid,
        AnimatedGridController, AnimatedItem, AnimatedItemBuilder, AnimatedItemPhase, AnimatedList,
        AnimatedListController, AnimatedModalBarrier, AnimatedOpacity, AnimatedPadding,
        AnimatedPhysicalModel, AnimatedPositioned, AnimatedPositionedDirectional,
        AnimatedRemovedItemBuilder, AnimatedRotation, AnimatedScale, AnimatedSize, AnimatedSlide,
        AnimatedSwitcher, AnimatedTransitionBuilder, AnnotatedRegion, ApplicationBootstrapHost,
        ApplicationBootstrapOptions, ApplicationBootstrapSpec, AspectRatio, AsyncSnapshot,
        AsyncWidgetBuilder, AutocompleteHighlightedOption, AutofillContextAction, AutofillGroup, AutomaticKeepAlive,
        AutovalidateMode, AuxiliaryViewError, AuxiliaryViewHandle, AuxiliaryViewHost,
        AuxiliaryViewOutcome, AuxiliaryViewRequest, BackButtonDispatcher, BackButtonListener,
        BackCallbackSubscription, BackHandlerResult, BackRegistration, Banner, BannerLocation,
        Baseline, BasicRouterDelegate, Border, BorderComposition, BorderDirectional, BorderRadius,
        BorderRadiusDirectional, BorderSide, BorderStyle, BoxBorder, BoxBorderLayer, BoxBorderStack, BoxDecoration, BoxFit,
        BoxLinearGradient, BoxShadow, BoxShape, Builder, CacheExtentStyle, CallbackShortcuts,
        Center, ChangeReportingBehavior, CheckedModeBanner, ChildBackButtonDispatcher,
        ChildVicinity, ClipOval, ClipPath, ClipRRect, ClipRSuperellipse, ClipRect, ClipShape,
        ClosureRouteInformationParser, ColorFiltered, ColoredBox, Column, Command, CommandId,
        ConnectionState, ConstrainedBox, ConstraintsTransformBox, Container, CreateRectTween,
        CrossFadeState, CustomClipper, CustomMultiChildLayout, CustomPaint, CustomPainter,
        CustomPainterSemantics, CustomScrollView, CustomSingleChildLayout, DecoratedBox,
        DecoratedBoxTransition, DecoratedShapeBox, DecoratedSliver, DecorationImage,
        DecorationPosition, DefaultAssetBundle, DefaultSelectionStyle, DefaultTextHeightBehavior,
        DefaultTextStyle, DefaultTextStyleTransition, DiagonalDragBehavior, Directionality,
        DismissDirection, DismissIntent, DismissMenuAction, DismissUpdateDetails, Dismissible, DisplayFeatureSubScreen,
        DragAnchorStrategy, DragBoundary, DragBoundaryDelegate, DragDropContext, DragEndDetails,
        DragStartDetails, DragTarget, DragUpdateDetails, Draggable, DraggableAnimationStep,
        DraggableDetails, DraggableNotificationSubscription, DraggableScrollableActuator,
        DraggableScrollableController, DraggableScrollableNotification, DraggableScrollableSheet,
        DraggableScrollableState, DraggableSheetDelta, DraggableSheetExtent,
        DraggableSizeAnimation, DraggableSnap, DraggableSnapTarget, DualTransitionBuilder,
        EditableText, ErrorWidget, ExcludeFocus, ExcludeFocusTraversal, Expanded, Expansible,
        FadeInImage, FadeTransition, FittedBox, Flex, Flexible, Flow, FlowDelegate,
        FlowPaintingContext, FlutterLogo, FlutterLogoLayout, FlutterLogoStyle, Focus, FocusManager,
        FocusNode, FocusScope, FocusScopeNode, FocusTraversalGroup, FocusTraversalOrder,
        FocusTraversalPolicy, FocusableActionDetector, Form, FormController, FormField,
        FormFieldState, FormState, FractionalTranslation, FractionallySizedBox, FutureBuilder,
        GestureDetector, GestureRecognizer, GestureRecognizerFactory,
        GestureRecognizerFactoryError, GestureRecognizerFactoryWithHandlers,
        GlowingOverscrollIndicator, GridPaper, GridView, Hero, HeroFlightDirection,
        HeroFlightShuttleBuilder, HeroMode, HeroPlaceholderBuilder, HeroRectTween,
        HeroShuttleContext, HeroTag, Icon, IconTheme, IgnoreBaseline, IgnorePointer, Image,
        ImageErrorBuilder, ImageFiltered, ImageFrameBuilder, ImageIcon, ImageLoadingBuilder,
        ImageRepeat, IndexedStack, InheritedModel, InheritedModelData, Intent, InteractiveViewer,
        KeepAlive, KeepAliveHandle, KeepAliveNotification, KeepAliveRegistry, KeyboardListener,
        KeyedSubtree, LayoutBuilder, LayoutChildId, LayoutId, LimitedBox, ListBody, ListView,
        ListWheelScrollView, ListWheelViewport, Listener, ListenerCallbacks, Localizations,
        LogicalShortcutKey, LongPressDraggable, LookupBoundary, MatrixTransition, MediaQuery,
        MediaQueryData, MemoryRouteInformationProvider, MetaData, MouseCursor, MouseRegion,
        MouseRegionCallbacks, MultiChildLayoutContext, MultiChildLayoutDelegate,
        NavigationBackButtonDispatcher, NavigationNotification, NavigationNotificationKind,
        NavigationNotificationListener, NavigationNotificationSubscription, NavigationToolbar,
        NavigatorPopHandler, NavigatorPopHandlerController, NestedScrollView,
        NoopAuxiliaryViewHost, NoopWindowChromeSink, NotificationDispatcher, NotificationListener,
        Offstage, Opacity, OrderedTraversalPolicy, Orientation, OrientationBuilder, OverflowBar,
        OverflowBox, OverlayPortal, Padding, PageController, PageMetrics, PageMetricsUpdate, PageStorage,
        PageStorageBucket,
        PageStorageIdentifier, PageStorageKey, PageView, PanAxis, PathView, PerformanceOverlay,
        PersistentHeaderShowOnScreenConfiguration, PhysicalModel, PhysicalShape, PinnedHeaderSliver,
        Placeholder, PointerSignalResolver,
        PopAttempt, PopScope, PopScopeController, Positioned, PositionedDirectional,
        PositionedTransition, PreferredSize, PrimaryScrollController, ProviderImage, RadioGroup,
        Radius, RawAutocomplete, RawGestureDetector, RawImage, RawMenuAnchor, RawMenuAnchorGroup,
        RawMenuOverlayInfo, RawPointerEvent, RawRadio, RawScrollbar, RawScrollbarGeometry,
        RawScrollbarOrientation, RawScrollbarStyle, RawTooltip, RawTooltipBuilder,
        RawTooltipController, RawTooltipDurations, RawTooltipTriggerMode, RawTooltipVisibility,
        ReadingOrderTraversalPolicy, RectDragBoundary, RelativePositionedTransition, RemovePadding,
        ReorderableDelayedDragStartListener, ReorderableDragStartListener, ReorderableList,
        RepaintBoundary, RepeatMode, RepeatingAnimationBuilder, RepeatingAnimationWidgetBuilder,
        RootBackButtonDispatcher, RootRestorationScope, RotatedBox, RotationTransition,
        RoundedCircleOutline, RoundedRectangleBorder, RoundedSuperellipseBorder, RouteInformation,
        RouteInformationListener, RouteInformationParser, RouteInformationProvider,
        RouteInformationReportingType, RouteInformationSubscription, Router, RouterConfig,
        RouterData, RouterDelegate, RouterDelegateListener, RouterDelegateSubscription,
        RouterError, Row, SafeArea, ScaleTransition, ScrollAwareContext, ScrollAwareImageProvider,
        ScrollBehavior, ScrollCacheExtent, ScrollConfiguration, ScrollController, ScrollMetrics,
        ScrollNotification,
        ScrollNotificationObserver, ScrollNotificationSubscription, ScrollNotificationType,
        ScrollPhysics, ScrollViewKeyboardDismissBehavior, SelectableRegion, Semantics,
        SensitiveContent, SensitiveContentHost, ShapeBorder, ShapeDecoration, SharedAppData,
        SharedAppDataKey, SharedAppDataStore, ShortcutActivator, ShortcutKey, ShortcutTrigger,
        Shortcuts, ShrinkWrappingViewport, SingleActivator, SingleChildLayoutDelegate,
        SingleChildScrollView, SizeChangedLayoutNotification, SizeChangedLayoutNotifier,
        SizeTransition, SizedBox, SizedOverflowBox, SlideTransition, Sliver, SliverAnimatedGrid,
        SliverAnimatedGridController, SliverAnimatedList, SliverAnimatedListController,
        SliverAnimatedOpacity, SliverChildBuilderDelegate, SliverChildDelegate,
        SliverChildListDelegate, SliverConstrainedCrossAxis, SliverConstraints,
        SliverCrossAxisExpanded, SliverCrossAxisGroup, SliverEnsureSemantics, SliverFadeTransition,
        SliverFillRemaining, SliverFillViewport, SliverFixedExtentList, SliverFloatingHeader,
        SliverGrowthDirection,
        SliverGrid, SliverGridDelegate, SliverLayout, SliverLayoutBuilder, SliverList,
        SliverMainAxisGroup, SliverOffstage, SliverOpacity, SliverOverlapAbsorber,
        SliverOverlapHandle, SliverOverlapInjector, SliverPadding, SliverPersistentHeader,
        SliverPrototypeExtentList, SliverReorderController, SliverReorderableList,
        SliverResizingHeader, SliverSafeArea, SliverSemantics, SliverToBoxAdapter,
        SliverVariedExtentList, SliverVisibility, SnapshotWidget, Spacer, Stack, StadiumBorder,
        StarBorder, StarBorderError, StarBorderUpdate, StreamBuilder, StreamBuilderBase, StreamEvent, StretchEffect,
        StretchingOverscrollIndicator, StringRouteInformationParser, Table, TableCell,
        TableBorder, TableCellVerticalAlignment, TableColumnWidth, TableRow, TableRowLengthError,
        TapRegion, TapRegionCallbacks,
        TapRegionGroupId, TapRegionSurface, Text, TextFieldTapRegion, TextInputActionHint,
        TextInputTypeHint, TickerMode, TileMode, Title, TitleController, TitleData, TitleError,
        ToolbarOptions, TooltipComponentBuilder, TooltipDurations, TooltipPositionContext,
        TooltipPositionDelegate, TooltipTriggerMode,
        TooltipVisibility, Transform, TransformationController, TreeRowAnimation, TreeSliver,
        TreeSliverController, TreeSliverIndentation, TreeSliverNode, TreeSliverNodeId,
        TweenAnimation, TweenAnimationBuilder, TweenAnimationWidgetBuilder,
        TwoDimensionalChildDelegate, TwoDimensionalChildLayout, TwoDimensionalConstraints,
        TwoDimensionalScrollDelta, TwoDimensionalScrollView, TwoDimensionalScrollable,
        TwoDimensionalViewport, TwoDimensionalViewportLayout, UiCompleter, UiFuture, UiStream,
        UiStreamController, UiStreamSubscription, UnconstrainedBox, UndoHistory,
        UnmanagedRestorationScope, View, ViewAnchor, ViewAnchorController, ViewAnchorData,
        ViewAnchorSubscription, ViewController, ViewData, ViewEvent, ViewId, ViewLifecycle,
        ViewMetrics, ViewSubscription, Viewport, Visibility, WheelChildDelegate, WheelChildLayout,
        WheelLayout, WheelMatrix, WheelProjection, Widget, WidgetOrderTraversalPolicy, WidgetsApp,
        WidgetsAppController, WidgetsAppData, WindowChromeSink, WindowDragRegion,
        WindowResizeRegion, Wrap, normalize_route_location,
    };
    pub use incular_widgets::{
        AnimatedBuilder, Listenable, ListenableBuilder, OpacityController, RelativeRect,
        RotationController, ScaleController, TransitionBuilder, TranslationController,
    };
}

/// Opt-in Controls convenience imports. Controls stay separate from the base
/// prelude so applications without the optional `controls` feature keep the
/// renderer-neutral API surface.
#[cfg(feature = "controls")]
pub mod controls_prelude {
    pub use incular_controls::prelude::*;
}

/// Opt-in Material convenience imports. Material is intentionally not part of
/// [`prelude`] so a base Widgets application cannot accidentally depend on a
/// design system. Enable the `material` feature and import this module when
/// using Material components.
#[cfg(feature = "material")]
pub mod material_prelude {
    pub use incular_material::prelude::*;
}

#[cfg(any(
    feature = "desktop-host",
    feature = "web-host",
    feature = "mobile-host"
))]
pub use incular_platform as platform;
pub use incular_core::app_build_version;

#[cfg(any(
    feature = "desktop-host",
    feature = "web-host",
    feature = "mobile-host"
))]
pub use incular_wgpu as wgpu;

#[cfg(all(
    feature = "desktop-host",
    not(feature = "desktop"),
    not(any(
        feature = "d3d11",
        feature = "dx12",
        feature = "metal",
        feature = "vulkan",
        feature = "gl"
    ))
))]
compile_error!(
    "`desktop-host` needs at least one GPU backend feature: d3d11, dx12, metal, vulkan or gl"
);

#[cfg(all(feature = "web-host", not(any(feature = "webgpu", feature = "webgl"))))]
compile_error!("`web-host` needs at least one browser GPU backend feature: webgpu or webgl");

/// Browser entry over the shared Winit host (`wasm32` targets).
#[cfg(feature = "web-host")]
pub use incular_web as web;

#[cfg(all(feature = "desktop-host", target_os = "linux"))]
pub use incular_linux as linux;

#[cfg(all(feature = "desktop-host", target_os = "macos"))]
pub use incular_macos as macos;
#[cfg(all(feature = "desktop-host", target_os = "windows"))]
pub use incular_windows as windows;

/// Runs an application on the native backend selected by the compilation target.
#[cfg(all(feature = "desktop-host", target_os = "linux"))]
pub fn run(application: incular_runtime::Application) -> Result<(), incular_linux::RunError> {
    incular_linux::run_application(application)
}
/// Runs the native Windows event loop on the calling main thread.
/// Returns typed backend errors.
#[cfg(all(feature = "desktop-host", target_os = "windows"))]
pub fn run(application: incular_runtime::Application) -> Result<(), incular_windows::RunError> {
    incular_windows::run_application(application)
}
/// Runs the native macOS event loop on the calling main thread.
/// Returns typed backend errors.
#[cfg(all(feature = "desktop-host", target_os = "macos"))]
pub fn run(application: incular_runtime::Application) -> Result<(), incular_macos::RunError> {
    incular_macos::run_application(application)
}

/// Starts the application in a canvas appended to the page. The page owns the
/// event loop, so this returns as soon as the host is installed.
#[cfg(all(feature = "web-host", target_arch = "wasm32"))]
pub fn run(application: incular_runtime::Application) -> Result<(), incular_web::RunError> {
    incular_web::run_application(application)
}

/// Android embedder entry: the activity runs the app's `main`, whose
/// [`run`] call hands the application to the activity.
#[cfg(all(feature = "mobile-host", target_os = "android"))]
pub use incular_android as android;

/// iOS embedder entry over UIKit.
#[cfg(all(feature = "mobile-host", target_os = "ios"))]
pub use incular_ios as ios;

/// Hands the application to the Android activity, which drives it. Returns
/// immediately; the activity owns the event loop.
#[cfg(all(feature = "mobile-host", target_os = "android"))]
pub fn run(application: incular_runtime::Application) -> Result<(), incular_android::RunError> {
    incular_android::run_application(application)
}

/// Runs the UIKit application on the calling main thread. Never returns on
/// success; UIKit owns the process.
#[cfg(all(feature = "mobile-host", target_os = "ios"))]
pub fn run(application: incular_runtime::Application) -> Result<(), incular_ios::RunError> {
    incular_ios::run_application(application)
}

pub use incular_runtime::PERFORMANCE_OVERLAY_KEY;

/// Formats a snapshot into standard overlay lines (thin wrapper over
/// [`incular_runtime::PerformanceSnapshot::overlay_lines`]).
#[must_use]
pub fn overlay_lines(snapshot: &runtime::PerformanceSnapshot) -> Vec<String> {
    snapshot.overlay_lines()
}

/// Installs the repaint-contained debug performance overlay into `window`.
/// The tree must contain a placeholder keyed with
/// [`PERFORMANCE_OVERLAY_KEY`]; publishing rebuilds only that element.
#[cfg(any(
    feature = "desktop-host",
    feature = "web-host",
    feature = "mobile-host"
))]
pub fn install_performance_overlay(
    application: &mut runtime::Application,
    window: platform::WindowId,
) -> Result<(), widgets::internal::TreeError> {
    application.install_performance_overlay(window)
}
