//! AppKit/UserNotifications application-shell integration.

use block2::Block;
use incular_desktop::{NativeTrays, ShellEventRegistration, ShellEventSink};
use incular_platform::{
    ApplicationShellError, ApplicationShellFeature, CapabilitySupport, NativeWindowSystem,
    PlatformCapabilities,
};
use incular_runtime::{
    ApplicationShellRequestId, NativeApplicationShellApplyResult, NativeApplicationShellCompletion,
    NativeApplicationShellEvent, NativeApplicationShellOperation, NativeApplicationShellRequest,
};
use objc2::{
    ClassType, DeclaredClass, declare_class, msg_send_id, mutability, rc::Retained,
    runtime::ProtocolObject,
};
use objc2_foundation::{MainThreadMarker, NSArray, NSObject, NSObjectProtocol, NSSet, NSString};
use objc2_user_notifications::{
    UNAuthorizationOptions, UNMutableNotificationContent, UNNotification, UNNotificationAction,
    UNNotificationActionOptionNone, UNNotificationCategory, UNNotificationCategoryOptions,
    UNNotificationDefaultActionIdentifier, UNNotificationDismissActionIdentifier,
    UNNotificationPresentationOptions, UNNotificationRequest, UNNotificationResponse,
    UNUserNotificationCenter, UNUserNotificationCenterDelegate,
};
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    sync::{Arc, Mutex},
};

type CompletionSink = Arc<dyn Fn(NativeApplicationShellCompletion) + Send + Sync>;

struct NotificationDelegateIvars {
    deliver: ShellEventSink,
}

declare_class!(
    struct IncularNotificationCenterDelegate;

    // SAFETY: NSObject has no extra subclassing invariants. UserNotifications
    // installs and calls this delegate from the application's main-thread
    // notification center, matching the MainThreadOnly mutability marker.
    unsafe impl ClassType for IncularNotificationCenterDelegate {
        type Super = NSObject;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "IncularNotificationCenterDelegate";
    }

    impl DeclaredClass for IncularNotificationCenterDelegate {
        type Ivars = NotificationDelegateIvars;
    }

    unsafe impl NSObjectProtocol for IncularNotificationCenterDelegate {}

    unsafe impl UNUserNotificationCenterDelegate for IncularNotificationCenterDelegate {
        #[method(userNotificationCenter:willPresentNotification:withCompletionHandler:)]
        fn will_present(
            &self,
            _center: &UNUserNotificationCenter,
            _notification: &UNNotification,
            completion_handler: &Block<dyn Fn(UNNotificationPresentationOptions)>,
        ) {
            completion_handler.call((
                UNNotificationPresentationOptions::UNNotificationPresentationOptionBanner
                    | UNNotificationPresentationOptions::UNNotificationPresentationOptionList
                    | UNNotificationPresentationOptions::UNNotificationPresentationOptionSound,
            ));
        }

        #[method(userNotificationCenter:didReceiveNotificationResponse:withCompletionHandler:)]
        fn did_receive(
            &self,
            _center: &UNUserNotificationCenter,
            response: &UNNotificationResponse,
            completion_handler: &Block<dyn Fn()>,
        ) {
            // SAFETY: UserNotifications retains the response for the callback;
            // its notification/request/action identifier accessors return
            // retained Foundation objects valid for this synchronous decode.
            let decoded = unsafe {
                let notification = response.notification();
                let request = notification.request();
                let identifier = request.identifier().to_string();
                let id = decode_notification_id(&identifier);
                let action = response.actionIdentifier();
                id.map(|id| {
                    if &*action == UNNotificationDefaultActionIdentifier {
                        NativeApplicationShellEvent::NotificationActivated { id }
                    } else if &*action == UNNotificationDismissActionIdentifier {
                        NativeApplicationShellEvent::NotificationDismissed { id }
                    } else {
                        NativeApplicationShellEvent::NotificationAction {
                            id,
                            action: incular_platform::NotificationActionId::new(
                                action.to_string(),
                            ),
                        }
                    }
                })
            };
            if let Some(event) = decoded {
                (self.ivars().deliver)(event);
            }
            completion_handler.call(());
        }
    }
);

impl IncularNotificationCenterDelegate {
    fn new(deliver: ShellEventSink, mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc();
        let this = this.set_ivars(NotificationDelegateIvars { deliver });
        // SAFETY: `this` is freshly allocated with all Rust ivars initialized;
        // NSObject's `init` is the designated initializer for this subclass.
        unsafe { msg_send_id![super(this), init] }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum AuthorizationState {
    #[default]
    Unknown,
    Granted,
    Denied,
}

#[derive(Clone, Default)]
pub(crate) struct MacosApplicationShell {
    inner: Rc<MacosApplicationShellInner>,
}

// Field order is drop order: the event sink is released after native resources.
#[derive(Default)]
struct MacosApplicationShellInner {
    trays: NativeTrays,
    categories:
        RefCell<HashMap<incular_platform::NotificationId, Retained<UNNotificationCategory>>>,
    notification_delegate: RefCell<Option<Retained<IncularNotificationCenterDelegate>>>,
    completion_sink: RefCell<Option<CompletionSink>>,
    authorization: Arc<Mutex<AuthorizationState>>,
    pending_authorization:
        Arc<Mutex<HashMap<incular_platform::NotificationId, ApplicationShellRequestId>>>,
    events: RefCell<Option<ShellEventRegistration>>,
}

impl Drop for MacosApplicationShellInner {
    fn drop(&mut self) {
        self.pending_authorization
            .lock()
            .expect("macOS pending notification authorization lock")
            .clear();
        self.categories.get_mut().clear();
        self.completion_sink.get_mut().take();
        if self.notification_delegate.get_mut().take().is_some() {
            // SAFETY: UserNotifications' center is process-global and removing
            // our retained delegate during application-shell teardown cannot
            // outlive any reference stored by this inner state.
            unsafe {
                UNUserNotificationCenter::currentNotificationCenter().setDelegate(None);
            }
        }
    }
}

impl MacosApplicationShell {
    pub(crate) fn refine_capabilities(
        &self,
        system: NativeWindowSystem,
        capabilities: &mut PlatformCapabilities,
    ) {
        let shell_supported = system == NativeWindowSystem::AppKit;
        let support = CapabilitySupport::from_supported(shell_supported);
        let unsupported = CapabilitySupport::Unsupported;
        let services = &mut capabilities.application_services;
        services.tray_or_status_item = support;
        services.notifications = support;
        services.notification_actions = support;
        services.notification_update = support;
        services.notification_dismiss = support;
        services.taskbar_progress = unsupported;
        services.application_badge = support;
        services.taskbar_overlay_icon = unsupported;
    }

    pub(crate) fn start_watch(&self, sink: ShellEventSink, complete: CompletionSink) {
        let registration = ShellEventRegistration::install(Arc::clone(&sink));
        *self.inner.events.borrow_mut() = Some(registration);
        *self.inner.completion_sink.borrow_mut() = Some(complete);

        if self.inner.notification_delegate.borrow().is_none() {
            let mtm = MainThreadMarker::new()
                .expect("macOS notification watch must be installed on the main thread");
            let delegate = IncularNotificationCenterDelegate::new(sink, mtm);
            // SAFETY: delegate is retained by our shell for at least as long as
            // it remains installed in the process-global notification center.
            unsafe {
                UNUserNotificationCenter::currentNotificationCenter()
                    .setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
            }
            *self.inner.notification_delegate.borrow_mut() = Some(delegate);
        }
    }

    pub(crate) fn apply(
        &self,
        system: NativeWindowSystem,
        request: NativeApplicationShellRequest,
        _target_window: Option<&winit::window::Window>,
    ) -> NativeApplicationShellApplyResult {
        if system != NativeWindowSystem::AppKit {
            return NativeApplicationShellApplyResult::Completed(Err(
                ApplicationShellError::Unsupported(request.operation.required_feature()),
            ));
        }
        let request_id = request.request_id;
        let trays = &self.inner.trays;
        match request.operation {
            NativeApplicationShellOperation::CreateTray {
                id,
                presentation,
                menu,
            } => trays.create(id, &presentation, &menu).into(),
            NativeApplicationShellOperation::UpdateTray {
                id,
                presentation,
                menu,
            } => trays.update(id, &presentation, &menu).into(),
            NativeApplicationShellOperation::RemoveTray { id } => trays.remove(id).into(),
            NativeApplicationShellOperation::ShowNotification { id, presentation } => {
                self.submit_notification(request_id, id, presentation)
            }
            NativeApplicationShellOperation::UpdateNotification { id, presentation } => {
                self.remove_notification_request(id);
                self.submit_notification(request_id, id, presentation)
            }
            NativeApplicationShellOperation::CloseNotification { id } => {
                // Synchronize with an authorization callback which may be
                // about to submit this exact stable notification ID. Holding
                // the pending map across native removal gives deterministic
                // ordering: either the callback submits first and we remove
                // it, or close removes the pending token and the callback is
                // rejected as stale before it can submit.
                let mut pending = self
                    .inner
                    .pending_authorization
                    .lock()
                    .expect("macOS pending notification authorization lock");
                pending.remove(&id);
                self.remove_notification_request(id);
                drop(pending);
                self.inner.categories.borrow_mut().remove(&id);
                self.publish_categories();
                NativeApplicationShellApplyResult::Completed(Ok(()))
            }
            NativeApplicationShellOperation::SetTaskbarDockState(state) => {
                NativeApplicationShellApplyResult::Completed(set_dock_state(state))
            }
        }
    }

    fn submit_notification(
        &self,
        request_id: ApplicationShellRequestId,
        id: incular_platform::NotificationId,
        presentation: incular_platform::NotificationPresentation,
    ) -> NativeApplicationShellApplyResult {
        let center = unsafe { UNUserNotificationCenter::currentNotificationCenter() };
        let category_identifier = match self.install_category(id, &presentation, &center) {
            Ok(identifier) => identifier,
            Err(error) => return NativeApplicationShellApplyResult::Completed(Err(error)),
        };

        let content = unsafe { UNMutableNotificationContent::new() };
        unsafe {
            content.setTitle(&NSString::from_str(&presentation.title));
            content.setBody(&NSString::from_str(&presentation.body));
            content.setCategoryIdentifier(&NSString::from_str(&category_identifier));
        }
        let identifier = NSString::from_str(&notification_identifier(id));
        let request = unsafe {
            UNNotificationRequest::requestWithIdentifier_content_trigger(
                &identifier,
                &content,
                None,
            )
        };
        let Some(complete) = self.inner.completion_sink.borrow().as_ref().cloned() else {
            return NativeApplicationShellApplyResult::Completed(Err(
                ApplicationShellError::NativeFailure(
                    "macOS notification completion sink is not installed".to_owned(),
                ),
            ));
        };

        let authorization_state = *self
            .inner
            .authorization
            .lock()
            .expect("macOS notification authorization lock");
        match authorization_state {
            AuthorizationState::Granted => {
                submit_notification_request(request_id, request, complete);
                NativeApplicationShellApplyResult::Deferred
            }
            AuthorizationState::Denied => NativeApplicationShellApplyResult::Completed(Err(
                ApplicationShellError::PermissionDenied,
            )),
            AuthorizationState::Unknown => {
                let authorization = Arc::clone(&self.inner.authorization);
                let pending_authorization = Arc::clone(&self.inner.pending_authorization);
                let authorization_complete = Arc::clone(&complete);
                self.inner
                    .pending_authorization
                    .lock()
                    .expect("macOS pending notification authorization lock")
                    .insert(id, request_id);
                let completion = block2::RcBlock::new(
                    move |granted: objc2::runtime::Bool, error: *mut objc2_foundation::NSError| {
                        let mut pending = pending_authorization
                            .lock()
                            .expect("macOS pending notification authorization lock");
                        if pending.get(&id).copied() != Some(request_id) {
                            authorization_complete(NativeApplicationShellCompletion {
                                request_id,
                                result: Err(ApplicationShellError::StaleResource),
                            });
                            return;
                        }
                        pending.remove(&id);
                        if granted.as_bool() {
                            *authorization
                                .lock()
                                .expect("macOS notification authorization lock") =
                                AuthorizationState::Granted;
                            submit_notification_request(
                                request_id,
                                request.clone(),
                                Arc::clone(&authorization_complete),
                            );
                            drop(pending);
                            return;
                        }

                        let result = if error.is_null() {
                            *authorization
                                .lock()
                                .expect("macOS notification authorization lock") =
                                AuthorizationState::Denied;
                            Err(ApplicationShellError::PermissionDenied)
                        } else {
                            *authorization
                                .lock()
                                .expect("macOS notification authorization lock") =
                                AuthorizationState::Unknown;
                            Err(ApplicationShellError::NativeFailure(
                                "macOS notification authorization failed".to_owned(),
                            ))
                        };
                        authorization_complete(NativeApplicationShellCompletion {
                            request_id,
                            result,
                        });
                        drop(pending);
                    },
                );
                unsafe {
                    center.requestAuthorizationWithOptions_completionHandler(
                        UNAuthorizationOptions::UNAuthorizationOptionAlert
                            | UNAuthorizationOptions::UNAuthorizationOptionBadge
                            | UNAuthorizationOptions::UNAuthorizationOptionSound,
                        &completion,
                    );
                }
                NativeApplicationShellApplyResult::Deferred
            }
        }
    }

    fn install_category(
        &self,
        id: incular_platform::NotificationId,
        presentation: &incular_platform::NotificationPresentation,
        _center: &UNUserNotificationCenter,
    ) -> Result<String, ApplicationShellError> {
        let identifier = notification_category_identifier(id);
        let native_actions = presentation
            .actions
            .iter()
            .map(|action| unsafe {
                UNNotificationAction::actionWithIdentifier_title_options(
                    &NSString::from_str(action.id.as_str()),
                    &NSString::from_str(&action.label),
                    UNNotificationActionOptionNone,
                )
            })
            .collect::<Vec<_>>();
        let actions = NSArray::from_id_slice(&native_actions);
        let intents = NSArray::<NSString>::from_id_slice(&[]);
        let category = unsafe {
            UNNotificationCategory::categoryWithIdentifier_actions_intentIdentifiers_options(
                &NSString::from_str(&identifier),
                &actions,
                &intents,
                UNNotificationCategoryOptions::UNNotificationCategoryOptionCustomDismissAction,
            )
        };
        self.inner.categories.borrow_mut().insert(id, category);
        self.publish_categories();
        Ok(identifier)
    }

    fn publish_categories(&self) {
        let categories = self
            .inner
            .categories
            .borrow()
            .values()
            .cloned()
            .collect::<Vec<_>>();
        let categories = NSArray::from_id_slice(&categories);
        // SAFETY: Cocoa's native `setWithArray:` retains and hashes these
        // UserNotifications category objects. The safe objc2 NSSet constructor
        // requires a Rust stable-hash marker which the interior-mutable
        // `UNNotificationCategory` binding intentionally does not implement.
        let categories = unsafe { NSSet::setWithArray(&categories) };
        unsafe {
            UNUserNotificationCenter::currentNotificationCenter()
                .setNotificationCategories(&categories);
        }
    }

    fn remove_notification_request(&self, id: incular_platform::NotificationId) {
        let identifier = NSString::from_str(&notification_identifier(id));
        let identifiers = NSArray::from_id_slice(&[identifier]);
        let center = unsafe { UNUserNotificationCenter::currentNotificationCenter() };
        unsafe {
            center.removePendingNotificationRequestsWithIdentifiers(&identifiers);
            center.removeDeliveredNotificationsWithIdentifiers(&identifiers);
        }
    }
}

fn submit_notification_request(
    request_id: ApplicationShellRequestId,
    request: Retained<UNNotificationRequest>,
    complete: CompletionSink,
) {
    let completion = block2::RcBlock::new(move |error: *mut objc2_foundation::NSError| {
        complete(NativeApplicationShellCompletion {
            request_id,
            result: if error.is_null() {
                Ok(())
            } else {
                Err(ApplicationShellError::NativeFailure(
                    "macOS rejected notification request".to_owned(),
                ))
            },
        });
    });
    unsafe {
        UNUserNotificationCenter::currentNotificationCenter()
            .addNotificationRequest_withCompletionHandler(&request, Some(&completion));
    }
}

fn set_dock_state(state: incular_platform::TaskbarDockState) -> Result<(), ApplicationShellError> {
    if state.progress != incular_platform::TaskbarProgress::None {
        return Err(ApplicationShellError::Unsupported(
            ApplicationShellFeature::TaskbarProgress,
        ));
    }
    if state.overlay_icon.is_some() {
        return Err(ApplicationShellError::Unsupported(
            ApplicationShellFeature::TaskbarOverlayIcon,
        ));
    }
    let mtm = MainThreadMarker::new().ok_or_else(|| {
        ApplicationShellError::NativeFailure(
            "macOS Dock state must be updated on the main thread".to_owned(),
        )
    })?;
    let application = objc2_app_kit::NSApplication::sharedApplication(mtm);
    let badge = match state.badge {
        incular_platform::ApplicationBadge::None => None,
        incular_platform::ApplicationBadge::Count(value) => Some(value.to_string()),
        incular_platform::ApplicationBadge::Text(value) => Some(value),
    };
    let badge = badge.as_deref().map(NSString::from_str);
    unsafe {
        application.dockTile().setBadgeLabel(badge.as_deref());
    }
    Ok(())
}

fn notification_identifier(id: incular_platform::NotificationId) -> String {
    format!("incular-notification:{}:{}", id.index(), id.generation())
}

fn notification_category_identifier(id: incular_platform::NotificationId) -> String {
    format!(
        "incular-notification-category:{}:{}",
        id.index(),
        id.generation()
    )
}

fn decode_notification_id(value: &str) -> Option<incular_platform::NotificationId> {
    let rest = value.strip_prefix("incular-notification:")?;
    let (index, generation) = rest.split_once(':')?;
    Some(incular_platform::NotificationId::from_parts(
        index.parse().ok()?,
        generation.parse().ok()?,
    ))
}
