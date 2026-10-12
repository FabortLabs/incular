//! Application-scoped native activation and global-shortcut vocabulary.
//!
//! These values deliberately contain no native window/application handles.
//! Desktop adapters receive OS events, normalize them here, and hand them to
//! the runtime's application-level delivery queue.

use crate::{DocumentActivation, DocumentDescriptor};
use incular_core::{Code, Modifiers};
use std::{ffi::OsString, sync::Arc};
use url::Url;

/// Arguments and typed resources associated with process launch.
///
/// `arguments` excludes the executable path and preserves the operating
/// system's native string representation. `documents` and `urls` are optional
/// semantic classifications supplied by a platform/packaging adapter; Incular
/// never guesses that an arbitrary command-line argument is a file or URL.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LaunchActivation {
    arguments: Arc<[OsString]>,
    documents: DocumentActivation,
    urls: Arc<[Url]>,
}

impl LaunchActivation {
    #[must_use]
    pub fn new(arguments: impl IntoIterator<Item = OsString>) -> Self {
        Self {
            arguments: arguments.into_iter().collect::<Vec<_>>().into(),
            ..Self::default()
        }
    }

    /// Captures the current process arguments without the executable name.
    #[must_use]
    pub fn current_process() -> Self {
        Self::new(std::env::args_os().skip(1))
    }

    #[must_use]
    pub fn with_documents(mut self, documents: DocumentActivation) -> Self {
        self.documents = documents;
        self
    }

    #[must_use]
    pub fn with_urls(mut self, urls: impl IntoIterator<Item = Url>) -> Self {
        self.urls = urls.into_iter().collect::<Vec<_>>().into();
        self
    }

    #[must_use]
    pub fn arguments(&self) -> &[OsString] {
        &self.arguments
    }

    #[must_use]
    pub const fn documents(&self) -> &DocumentActivation {
        &self.documents
    }

    #[must_use]
    pub fn urls(&self) -> &[Url] {
        &self.urls
    }
}

/// Ordered URLs delivered by the operating system or a secondary instance.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UrlActivation {
    urls: Arc<[Url]>,
}

impl UrlActivation {
    #[must_use]
    pub fn new(urls: impl IntoIterator<Item = Url>) -> Self {
        Self {
            urls: urls.into_iter().collect::<Vec<_>>().into(),
        }
    }

    #[must_use]
    pub fn urls(&self) -> &[Url] {
        &self.urls
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.urls.is_empty()
    }
}

/// Stable application-owned identity for one native global shortcut.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GlobalShortcutId(u64);

impl GlobalShortcutId {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Physical key chord registered with the operating system.
///
/// Global shortcuts intentionally use the same standardized physical [`Code`]
/// and [`Modifiers`] vocabulary as focused keyboard input, while retaining a
/// separate registration lifecycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GlobalShortcutChord {
    key: Code,
    modifiers: Modifiers,
}

impl GlobalShortcutChord {
    pub fn new(key: Code, modifiers: Modifiers) -> Result<Self, GlobalShortcutChordError> {
        let supported = Modifiers::ALT | Modifiers::CONTROL | Modifiers::META | Modifiers::SHIFT;
        if modifiers.bits() & !supported.bits() != 0 {
            return Err(GlobalShortcutChordError::UnsupportedModifiers(modifiers));
        }
        if key == Code::Unidentified || is_modifier_code(key) {
            return Err(GlobalShortcutChordError::InvalidKey(key));
        }
        Ok(Self { key, modifiers })
    }

    #[must_use]
    pub const fn key(self) -> Code {
        self.key
    }

    #[must_use]
    pub const fn modifiers(self) -> Modifiers {
        self.modifiers
    }
}

fn is_modifier_code(code: Code) -> bool {
    matches!(
        code,
        Code::AltLeft
            | Code::AltRight
            | Code::ControlLeft
            | Code::ControlRight
            | Code::MetaLeft
            | Code::MetaRight
            | Code::ShiftLeft
            | Code::ShiftRight
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum GlobalShortcutChordError {
    #[error("{0} cannot be a global shortcut key")]
    InvalidKey(Code),
    #[error("global shortcut modifiers {0:?} contain unsupported modifier state")]
    UnsupportedModifiers(Modifiers),
}

/// Typed failure from application-level global shortcut registration.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum GlobalShortcutError {
    #[error("global shortcuts are unsupported")]
    Unsupported,
    #[error("invalid global shortcut: {0}")]
    InvalidChord(GlobalShortcutChordError),
    #[error("global shortcut {} is already registered", .0.get())]
    AlreadyRegistered(GlobalShortcutId),
    #[error("global shortcut {} is not registered", .0.get())]
    NotRegistered(GlobalShortcutId),
    #[error(
        "global shortcut {:?}+{} conflicts with another registration",
        .0.modifiers(),
        .0.key()
    )]
    Conflict(GlobalShortcutChord),
    #[error("global shortcut application service stopped")]
    ApplicationStopped,
    #[error("global shortcut backend failed: {0}")]
    Backend(String),
}

impl From<GlobalShortcutChordError> for GlobalShortcutError {
    fn from(value: GlobalShortcutChordError) -> Self {
        Self::InvalidChord(value)
    }
}

/// Native/application events which are not owned by a particular window's
/// input stream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApplicationActivation {
    Launch(LaunchActivation),
    OpenFiles(DocumentActivation),
    OpenUrls(UrlActivation),
    Reopen,
    GlobalShortcutInvoked(GlobalShortcutId),
}

impl ApplicationActivation {
    #[must_use]
    pub fn open_files(paths: impl IntoIterator<Item = impl Into<std::path::PathBuf>>) -> Self {
        Self::OpenFiles(DocumentActivation::from_paths(paths))
    }

    #[must_use]
    pub fn open_urls(urls: impl IntoIterator<Item = Url>) -> Self {
        Self::OpenUrls(UrlActivation::new(urls))
    }

    #[must_use]
    pub fn documents(&self) -> &[DocumentDescriptor] {
        match self {
            Self::Launch(launch) => launch.documents().documents(),
            Self::OpenFiles(documents) => documents.documents(),
            Self::OpenUrls(_) | Self::Reopen | Self::GlobalShortcutInvoked(_) => &[],
        }
    }

    #[must_use]
    pub fn urls(&self) -> &[Url] {
        match self {
            Self::Launch(launch) => launch.urls(),
            Self::OpenUrls(urls) => urls.urls(),
            Self::OpenFiles(_) | Self::Reopen | Self::GlobalShortcutInvoked(_) => &[],
        }
    }
}

/// Opt-in policy selecting one primary process for an application identity.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SingleInstancePolicy {
    application_id: Arc<str>,
}

impl SingleInstancePolicy {
    pub fn new(application_id: impl Into<String>) -> Result<Self, SingleInstancePolicyError> {
        let application_id = application_id.into();
        let valid = !application_id.is_empty()
            && application_id.len() <= 128
            && application_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'));
        if !valid {
            return Err(SingleInstancePolicyError::InvalidApplicationId(
                application_id,
            ));
        }
        Ok(Self {
            application_id: Arc::from(application_id),
        })
    }

    #[must_use]
    pub fn application_id(&self) -> &str {
        &self.application_id
    }
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SingleInstancePolicyError {
    #[error(
        "single-instance application id {0:?} must be 1-128 ASCII letters, digits, '.', '_' or '-'"
    )]
    InvalidApplicationId(String),
}
