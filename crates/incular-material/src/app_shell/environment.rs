use incular_config::TargetPlatform;
use incular_core::PointerDeviceKind;
use incular_widgets::{
    AndroidOverscrollIndicator, ScrollBehavior, ScrollConfiguration, ScrollPhysics, Widget,
};
use smart_default::SmartDefault;
use typed_builder::TypedBuilder;

/// The input device classes accepted by the Material scroll behavior.
///
/// The retained gesture layer performs the actual dispatch.  Keeping this
/// policy as data lets platform adapters add/remove device classes without
/// coupling the Material crate to a native event enum.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum MaterialPointerDevice {
    #[default]
    Touch,
    Mouse,
    Trackpad,
    /// A stylus, either end.
    Stylus,
    Unknown,
}

impl MaterialPointerDevice {
    fn kinds(self) -> &'static [PointerDeviceKind] {
        match self {
            Self::Touch => &[PointerDeviceKind::Touch],
            Self::Mouse => &[PointerDeviceKind::Mouse],
            Self::Trackpad => &[PointerDeviceKind::Trackpad],
            Self::Stylus => &[PointerDeviceKind::Stylus, PointerDeviceKind::InvertedStylus],
            Self::Unknown => &[PointerDeviceKind::Unknown],
        }
    }
}

/// Flutter's `MaterialScrollBehavior`: the base `ScrollBehavior` with the
/// theme's platform, a scrollbar on vertical desktop scroll views and, on
/// Android, the Material 3 stretch.
///
/// Unset physics follow the platform; the drag devices default to Flutter's
/// touch-like set (touch, stylus, trackpad and unknown devices, not the
/// mouse).
#[derive(Clone, Debug, PartialEq, TypedBuilder, SmartDefault)]
pub struct MaterialScrollBehavior {
    #[builder(default, setter(strip_option))]
    physics: Option<ScrollPhysics>,
    #[default(true)]
    #[builder(default = true)]
    scrollbars: bool,
    #[default(vec![
        MaterialPointerDevice::Touch,
        MaterialPointerDevice::Stylus,
        MaterialPointerDevice::Trackpad,
        MaterialPointerDevice::Unknown,
    ])]
    #[builder(
        default = vec![
            MaterialPointerDevice::Touch,
            MaterialPointerDevice::Stylus,
            MaterialPointerDevice::Trackpad,
            MaterialPointerDevice::Unknown,
        ],
        setter(transform = |devices: impl IntoIterator<Item = MaterialPointerDevice>| {
            devices.into_iter().collect::<Vec<_>>()
        })
    )]
    drag_devices: Vec<MaterialPointerDevice>,
    /// The theme's platform; unset, the host's.
    #[builder(default, setter(strip_option))]
    platform: Option<TargetPlatform>,
}

impl MaterialScrollBehavior {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Physics overriding the platform's.
    #[must_use]
    pub fn physics(mut self, physics: ScrollPhysics) -> Self {
        self.physics = Some(physics);
        self
    }

    #[must_use]
    pub fn scrollbars(mut self, enabled: bool) -> Self {
        self.scrollbars = enabled;
        self
    }

    #[must_use]
    pub fn drag_devices(
        mut self,
        devices: impl IntoIterator<Item = MaterialPointerDevice>,
    ) -> Self {
        self.drag_devices = devices.into_iter().collect();
        self
    }

    /// The platform whose conventions apply, Flutter's `Theme.platform`.
    #[must_use]
    pub fn platform(mut self, platform: TargetPlatform) -> Self {
        self.platform = Some(platform);
        self
    }

    /// The physics in effect: the override, else the platform's (Flutter's
    /// `ScrollBehavior.getScrollPhysics`).
    #[must_use]
    pub fn get_physics(&self) -> ScrollPhysics {
        self.scroll_behavior().get_scroll_physics()
    }

    #[must_use]
    pub fn get_scrollbars(&self) -> bool {
        self.scrollbars
    }

    #[must_use]
    pub fn get_drag_devices(&self) -> &[MaterialPointerDevice] {
        &self.drag_devices
    }

    /// Flutter's `buildOverscrollIndicator`: on Android the stretch under
    /// Material 3 and the glow otherwise, on Fuchsia the glow, elsewhere
    /// none.
    #[must_use]
    pub fn overscroll_indicator(&self, use_material3: bool) -> Option<AndroidOverscrollIndicator> {
        match self.scroll_behavior().get_platform() {
            TargetPlatform::Android if use_material3 => Some(AndroidOverscrollIndicator::Stretch),
            TargetPlatform::Android | TargetPlatform::Fuchsia => {
                Some(AndroidOverscrollIndicator::Glow)
            }
            TargetPlatform::Ios
            | TargetPlatform::Linux
            | TargetPlatform::MacOs
            | TargetPlatform::Windows => None,
        }
    }

    /// The widgets-layer behavior this Material behavior configures.
    #[must_use]
    pub fn scroll_behavior(&self) -> ScrollBehavior {
        let mut behavior = ScrollBehavior::new()
            .scrollbars(self.scrollbars)
            .horizontal_scrollbars(false)
            .drag_devices(
                self.drag_devices
                    .iter()
                    .flat_map(|device| device.kinds().iter().copied()),
            );
        if let Some(physics) = self.physics {
            behavior = behavior.physics(physics);
        }
        if let Some(platform) = self.platform {
            behavior = behavior.platform(platform);
        }
        behavior
    }

    /// Wraps a subtree in a [`ScrollConfiguration`] with this behavior.
    #[must_use]
    pub fn wrap(&self, child: impl Into<Widget>) -> Widget {
        ScrollConfiguration::new(child)
            .behavior(self.scroll_behavior())
            .into()
    }
}
