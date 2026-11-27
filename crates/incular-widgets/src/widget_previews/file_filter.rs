//! Selected-file filtering for previews opened from a source editor.

use incular_core::reactivity::DependencySource;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

/// The editor's path rules, which may differ from the preview host's OS.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewPathStyle {
    Posix,
    Windows,
}

#[derive(Debug)]
pub struct PreviewFileEntry<T> {
    pub source_uri: String,
    pub group: String,
    pub value: T,
}

#[derive(Debug)]
pub struct PreviewFileGroup<T> {
    pub name: String,
    pub previews: Vec<Rc<PreviewFileEntry<T>>>,
}

type SourceEntry<T> = (Option<String>, Rc<PreviewFileEntry<T>>);

struct FilterState<T> {
    editor_available: bool,
    enabled: bool,
    selected: Option<String>,
    entries: Vec<SourceEntry<T>>,
    visible: Rc<[PreviewFileGroup<T>]>,
}

/// Retains the user's filter preference, editor readiness, and visible groups.
/// The preview host supplies its storage and the editor supplies file URIs.
pub struct PreviewFileFilter<T> {
    path_style: PreviewPathStyle,
    state: Rc<RefCell<FilterState<T>>>,
    changes: DependencySource,
}

impl<T> Clone for PreviewFileFilter<T> {
    fn clone(&self) -> Self {
        Self {
            path_style: self.path_style,
            state: self.state.clone(),
            changes: self.changes.clone(),
        }
    }
}

impl<T> PreviewFileFilter<T> {
    pub const PREFERENCE_KEY: &'static str = "filterBySelectedFile";

    #[must_use]
    pub fn new(
        path_style: PreviewPathStyle,
        editor_available: bool,
        stored_preference: Option<bool>,
        entries: impl IntoIterator<Item = PreviewFileEntry<T>>,
    ) -> Self {
        let mut state = FilterState {
            editor_available,
            enabled: stored_preference.unwrap_or(true),
            selected: None,
            entries: collect_entries(path_style, entries),
            visible: Rc::from([]),
        };
        refresh(&mut state, false);
        Self {
            path_style,
            state: Rc::new(RefCell::new(state)),
            changes: DependencySource::default(),
        }
    }

    #[must_use]
    pub fn toggle_visible(&self) -> bool {
        self.changes.track();
        self.state.borrow().editor_available
    }

    #[must_use]
    pub fn enabled(&self) -> bool {
        self.changes.track();
        self.state.borrow().enabled
    }

    #[must_use]
    pub fn visible_groups(&self) -> Rc<[PreviewFileGroup<T>]> {
        self.changes.track();
        self.state.borrow().visible.clone()
    }

    pub fn set_editor_available(&self, available: bool) {
        let mut state = self.state.borrow_mut();
        if state.editor_available == available {
            return;
        }
        state.editor_available = available;
        refresh(&mut state, true);
        drop(state);
        self.changes.notify();
    }

    /// An empty selection means the editor focused a non-source view. It
    /// must not flicker the existing preview groups out of the viewport.
    pub fn set_selected_source(&self, uri: Option<&str>) {
        let mut state = self.state.borrow_mut();
        let selected = uri.and_then(|uri| normalize_file_uri(self.path_style, uri));
        if state.selected == selected {
            return;
        }
        state.selected = selected;
        refresh(&mut state, false);
        drop(state);
        self.changes.notify();
    }

    pub fn replace_previews(&self, entries: impl IntoIterator<Item = PreviewFileEntry<T>>) {
        let mut state = self.state.borrow_mut();
        state.entries = collect_entries(self.path_style, entries);
        refresh(&mut state, false);
        drop(state);
        self.changes.notify();
    }

    /// Writes the changed preference before publishing it. Reading the
    /// default does not create a preference entry.
    pub fn toggle<E>(
        &self,
        persist: impl FnOnce(&'static str, bool) -> Result<(), E>,
    ) -> Result<(), E> {
        let next = !self.state.borrow().enabled;
        persist(Self::PREFERENCE_KEY, next)?;
        let mut state = self.state.borrow_mut();
        state.enabled = next;
        refresh(&mut state, false);
        drop(state);
        self.changes.notify();
        Ok(())
    }
}

fn collect_entries<T>(
    style: PreviewPathStyle,
    entries: impl IntoIterator<Item = PreviewFileEntry<T>>,
) -> Vec<SourceEntry<T>> {
    entries
        .into_iter()
        .map(|entry| (normalize_file_uri(style, &entry.source_uri), Rc::new(entry)))
        .collect()
}

fn refresh<T>(state: &mut FilterState<T>, editor_availability_changed: bool) {
    let selected = if state.editor_available && state.enabled {
        let Some(selected) = state.selected.as_deref() else {
            if editor_availability_changed {
                state.visible = Rc::from([]);
            }
            return;
        };
        Some(selected)
    } else {
        None
    };
    let mut positions = HashMap::new();
    let mut groups: Vec<PreviewFileGroup<T>> = Vec::new();
    for (source, entry) in &state.entries {
        if selected.is_some() && selected != source.as_deref() {
            continue;
        }
        let index = *positions.entry(entry.group.as_str()).or_insert_with(|| {
            let index = groups.len();
            groups.push(PreviewFileGroup {
                name: entry.group.clone(),
                previews: Vec::new(),
            });
            index
        });
        groups[index].previews.push(entry.clone());
    }
    state.visible = Rc::from(groups);
}

fn normalize_file_uri(style: PreviewPathStyle, uri: &str) -> Option<String> {
    let tail = uri.strip_prefix("file:")?;
    let parts = tail.split(['?', '#']).next()?;
    let (authority, path) = if let Some(rest) = parts.strip_prefix("//") {
        let slash = rest.find('/')?;
        (&rest[..slash], &rest[slash..])
    } else {
        ("", parts)
    };
    if !path.starts_with('/') {
        return None;
    }
    let mut path = percent_decode(path)?;
    if style == PreviewPathStyle::Windows {
        path = path.replace('\\', "/");
    }
    let remote = !authority.is_empty() && !authority.eq_ignore_ascii_case("localhost");
    let mut segments = Vec::new();
    if remote {
        segments.push(authority);
    }
    let windows_drive = style == PreviewPathStyle::Windows
        && !remote
        && path
            .trim_start_matches('/')
            .as_bytes()
            .get(..2)
            .is_some_and(|root| root[0].is_ascii_alphabetic() && root[1] == b':');
    let protected = if style == PreviewPathStyle::Windows && remote {
        2
    } else {
        usize::from(remote || windows_drive)
    };
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." if segments.len() > protected => {
                segments.pop();
            }
            ".." => {}
            _ => segments.push(segment),
        }
    }
    let normalized = format!("{}/{}", if remote { "/" } else { "" }, segments.join("/"));
    Some(match style {
        PreviewPathStyle::Posix => normalized,
        PreviewPathStyle::Windows => normalized.to_lowercase(),
    })
}

fn percent_decode(path: &str) -> Option<String> {
    let bytes = path.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b'%' {
            let high = char::from(*bytes.get(index + 1)?).to_digit(16)?;
            let low = char::from(*bytes.get(index + 2)?).to_digit(16)?;
            decoded.push((high * 16 + low) as u8);
            index += 3;
        } else {
            decoded.push(byte);
            index += 1;
        }
    }
    String::from_utf8(decoded)
        .ok()
        .filter(|s| !s.contains('\0'))
}
