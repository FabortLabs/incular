//! Flutter's `ScrollBehavior` (`widgets/scroll_configuration.dart`): the
//! platform-dependent defaults a [`ScrollConfiguration`] gives descendant
//! scroll views.
//!
//! [`ScrollConfiguration`]: crate::ScrollConfiguration

use incular_config::TargetPlatform;
use incular_core::PointerDeviceKind;
use incular_gestures::VelocityTrackerKind;
use incular_scroll::{ScrollDecelerationRate, ScrollPhysics, ScrollViewKeyboardDismissBehavior};

/// Flutter's `_kTouchLikeDeviceTypes`: the devices that drag-scroll by
/// default. The mouse is not one of them.
pub const TOUCH_LIKE_DEVICE_KINDS: [PointerDeviceKind; 5] = [
    PointerDeviceKind::Touch,
    PointerDeviceKind::Stylus,
    PointerDeviceKind::InvertedStylus,
    PointerDeviceKind::Trackpad,
    PointerDeviceKind::Unknown,
];

/// Flutter's `MultitouchDragStrategy`: how a drag combines several pointers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MultitouchDragStrategy {
    /// Only the latest active pointer moves the scroll view.
    LatestPointer,
    /// The extreme movements of all pointers in each direction combine.
    AverageBoundaryPointers,
    /// All pointers' movements add up.
    SumAllPointers,
}

/// Flutter's `AndroidOverscrollIndicator`: the indicator shown when an
/// Android scroll view is pulled past its edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AndroidOverscrollIndicator {
    /// The content stretches (Material 3).
    Stretch,
    /// A glow appears at the edge.
    Glow,
}

/// Flutter's `ScrollBehavior`, with its `copyWith` overrides as optional
/// fields: unset values follow the target platform.
#[derive(Clone, Debug, PartialEq)]
pub struct ScrollBehavior {
    scrollbars: bool,
    horizontal_scrollbars: bool,
    overscroll: bool,
    drag_devices: Option<Vec<PointerDeviceKind>>,
    multitouch_drag_strategy: Option<MultitouchDragStrategy>,
    physics: Option<ScrollPhysics>,
    platform: Option<TargetPlatform>,
    keyboard_dismiss_behavior: Option<ScrollViewKeyboardDismissBehavior>,
}

impl Default for ScrollBehavior {
    fn default() -> Self {
        Self::new()
    }
}

impl ScrollBehavior {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            scrollbars: true,
            horizontal_scrollbars: true,
            overscroll: true,
            drag_devices: None,
            multitouch_drag_strategy: None,
            physics: None,
            platform: None,
            keyboard_dismiss_behavior: None,
        }
    }

    /// Whether scroll views get the platform's scrollbar. Defaults to true.
    #[must_use]
    pub fn scrollbars(mut self, scrollbars: bool) -> Self {
        self.scrollbars = scrollbars;
        self
    }

    /// Whether horizontal scroll views get a scrollbar too. Defaults to
    /// true; Material's `buildScrollbar` gives only vertical ones a
    /// scrollbar.
    #[must_use]
    pub fn horizontal_scrollbars(mut self, horizontal: bool) -> Self {
        self.horizontal_scrollbars = horizontal;
        self
    }

    /// Whether scroll views get the platform's overscroll indicator.
    /// Defaults to true.
    #[must_use]
    pub fn overscroll(mut self, overscroll: bool) -> Self {
        self.overscroll = overscroll;
        self
    }

    /// The devices that may drag-scroll. Defaults to
    /// [`TOUCH_LIKE_DEVICE_KINDS`].
    #[must_use]
    pub fn drag_devices(mut self, devices: impl IntoIterator<Item = PointerDeviceKind>) -> Self {
        self.drag_devices = Some(devices.into_iter().collect());
        self
    }

    #[must_use]
    pub fn multitouch_drag_strategy(mut self, strategy: MultitouchDragStrategy) -> Self {
        self.multitouch_drag_strategy = Some(strategy);
        self
    }

    /// Physics for scroll views that set none of their own.
    #[must_use]
    pub fn physics(mut self, physics: ScrollPhysics) -> Self {
        self.physics = Some(physics);
        self
    }

    /// The platform whose conventions apply. Defaults to the host's.
    #[must_use]
    pub fn platform(mut self, platform: TargetPlatform) -> Self {
        self.platform = Some(platform);
        self
    }

    #[must_use]
    pub fn keyboard_dismiss_behavior(
        mut self,
        behavior: ScrollViewKeyboardDismissBehavior,
    ) -> Self {
        self.keyboard_dismiss_behavior = Some(behavior);
        self
    }

    /// Flutter's `getPlatform`: the override, else `defaultTargetPlatform`.
    #[must_use]
    pub fn get_platform(&self) -> TargetPlatform {
        self.platform.unwrap_or_else(TargetPlatform::host)
    }

    /// Flutter's `dragDevices`.
    #[must_use]
    pub fn get_drag_devices(&self) -> &[PointerDeviceKind] {
        self.drag_devices
            .as_deref()
            .unwrap_or(&TOUCH_LIKE_DEVICE_KINDS)
    }

    /// Whether a drag by `kind` scrolls.
    #[must_use]
    pub fn accepts_drag(&self, kind: PointerDeviceKind) -> bool {
        self.get_drag_devices().contains(&kind)
    }

    /// Flutter's `getMultitouchDragStrategy`: average boundary pointers on
    /// macOS and iOS, the latest pointer elsewhere.
    #[must_use]
    pub fn get_multitouch_drag_strategy(&self) -> MultitouchDragStrategy {
        self.multitouch_drag_strategy
            .unwrap_or(if self.get_platform().is_apple() {
                MultitouchDragStrategy::AverageBoundaryPointers
            } else {
                MultitouchDragStrategy::LatestPointer
            })
    }

    /// Flutter's `getScrollPhysics`: bouncing on iOS, bouncing with the fast
    /// deceleration rate on macOS, clamping elsewhere, each over
    /// `RangeMaintainingScrollPhysics`.
    #[must_use]
    pub fn get_scroll_physics(&self) -> ScrollPhysics {
        self.physics.unwrap_or_else(|| match self.get_platform() {
            TargetPlatform::Ios => ScrollPhysics::clamping().bouncing().range_maintaining(),
            TargetPlatform::MacOs => ScrollPhysics::clamping()
                .bouncing_with(ScrollDecelerationRate::Fast)
                .range_maintaining(),
            TargetPlatform::Android
            | TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::Windows => ScrollPhysics::clamping().range_maintaining(),
        })
    }

    /// Flutter's `buildScrollbar`: desktop platforms show a scrollbar.
    #[must_use]
    pub fn shows_scrollbar(&self) -> bool {
        self.scrollbars && self.get_platform().is_desktop()
    }

    /// Flutter's `buildOverscrollIndicator`: a glow on Android and Fuchsia.
    #[must_use]
    pub fn overscroll_indicator(&self) -> Option<AndroidOverscrollIndicator> {
        (self.overscroll
            && matches!(
                self.get_platform(),
                TargetPlatform::Android | TargetPlatform::Fuchsia
            ))
        .then_some(AndroidOverscrollIndicator::Glow)
    }

    /// Flutter's `velocityTrackerBuilder`.
    #[must_use]
    pub fn velocity_tracker(&self) -> VelocityTrackerKind {
        match self.get_platform() {
            TargetPlatform::Ios => VelocityTrackerKind::IosScrollViewFling,
            TargetPlatform::MacOs => VelocityTrackerKind::MacOsScrollViewFling,
            TargetPlatform::Android
            | TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::Windows => VelocityTrackerKind::LeastSquares,
        }
    }

    /// Flutter's `getKeyboardDismissBehavior`. Defaults to manual.
    #[must_use]
    pub fn get_keyboard_dismiss_behavior(&self) -> ScrollViewKeyboardDismissBehavior {
        self.keyboard_dismiss_behavior.unwrap_or_default()
    }

    pub(crate) fn resolve(&self) -> ResolvedScrollBehavior {
        let mut drag_devices = 0_u8;
        for kind in self.get_drag_devices() {
            drag_devices |= device_bit(*kind);
        }
        ResolvedScrollBehavior {
            drag_devices,
            scrollbar: self.shows_scrollbar(),
            horizontal_scrollbar: self.horizontal_scrollbars,
            velocity_tracker: self.velocity_tracker(),
        }
    }
}

const fn device_bit(kind: PointerDeviceKind) -> u8 {
    1 << match kind {
        PointerDeviceKind::Mouse => 0,
        PointerDeviceKind::Touch => 1,
        PointerDeviceKind::Stylus => 2,
        PointerDeviceKind::InvertedStylus => 3,
        PointerDeviceKind::Trackpad => 4,
        PointerDeviceKind::Unknown => 5,
    }
}

/// What a retained scroll view keeps of its ambient [`ScrollBehavior`].
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResolvedScrollBehavior {
    drag_devices: u8,
    pub(crate) scrollbar: bool,
    pub(crate) horizontal_scrollbar: bool,
    pub(crate) velocity_tracker: VelocityTrackerKind,
}

impl ResolvedScrollBehavior {
    pub(crate) const fn accepts_drag(self, kind: PointerDeviceKind) -> bool {
        self.drag_devices & device_bit(kind) != 0
    }
}

impl Default for ResolvedScrollBehavior {
    fn default() -> Self {
        ScrollBehavior::new().resolve()
    }
}
