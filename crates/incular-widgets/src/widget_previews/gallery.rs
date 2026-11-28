//! Design-system-neutral host for a snapshot of programmatically discovered previews.

use std::rc::Rc;

use incular_config::MainAxisSize;
use incular_semantics::Role;

use super::Preview;
use crate::{Column, ExcludeSemantics, GestureDetector, HitTestBehavior, Semantics, Text, Widget};

/// A rendered widget and the preview metadata that configures its embedding.
#[derive(Clone)]
pub struct PreviewGalleryEntry {
    pub preview: Preview,
    pub child: Widget,
}

impl PreviewGalleryEntry {
    #[must_use]
    pub fn new(preview: Preview, child: impl Into<Widget>) -> Self {
        Self {
            preview,
            child: child.into(),
        }
    }
}

type OpenDocumentation = Rc<dyn Fn(&str)>;

/// Groups the current preview snapshot or shows documentation if it is empty.
/// A hot-reload host supplies a new snapshot; this widget does not open a server
/// or perform source discovery. The documentation opener belongs to the host.
#[derive(Clone)]
pub struct PreviewGallery {
    entries: Vec<PreviewGalleryEntry>,
    documentation_url: Rc<str>,
    on_open_documentation: Option<OpenDocumentation>,
}

impl PreviewGallery {
    #[must_use]
    pub fn new(
        entries: impl IntoIterator<Item = PreviewGalleryEntry>,
        documentation_url: impl Into<Rc<str>>,
    ) -> Self {
        Self {
            entries: entries.into_iter().collect(),
            documentation_url: documentation_url.into(),
            on_open_documentation: None,
        }
    }

    #[must_use]
    pub fn on_open_documentation(mut self, callback: impl Fn(&str) + 'static) -> Self {
        self.on_open_documentation = Some(Rc::new(callback));
        self
    }

    fn documentation(self) -> Widget {
        let url = self.documentation_url;
        let label = Text::new(url.as_ref());
        let link = if let Some(open) = self.on_open_documentation {
            let gesture_url = url.clone();
            let gesture_open = open.clone();
            let action_url = url.clone();
            Semantics::new(
                GestureDetector::new(label)
                    .behavior(HitTestBehavior::Opaque)
                    .on_tap(move || gesture_open(&gesture_url)),
            )
            .on_tap(move || open(&action_url))
        } else {
            Semantics::new(label)
        };
        Column::new([
            Widget::from(Text::new("No previews detected")),
            Text::new("Read more about getting started with widget previews at:").into(),
            link.role(Role::Link)
                .label(url.as_ref())
                .link_url(url.as_ref())
                .into(),
        ])
        .main_axis_size(MainAxisSize::Min)
        .into()
    }
}

impl From<PreviewGallery> for Widget {
    fn from(gallery: PreviewGallery) -> Self {
        if gallery.entries.is_empty() {
            return gallery.documentation();
        }
        let mut groups = Vec::<(String, Vec<Widget>)>::new();
        for entry in gallery.entries {
            let name = entry.preview.group();
            let index = groups
                .iter()
                .position(|(group, _)| group == name)
                .unwrap_or_else(|| {
                    groups.push((name.to_owned(), Vec::new()));
                    groups.len() - 1
                });
            let child = entry.preview.compose(entry.child);
            let child = if let Some(title) = entry.preview.name() {
                Column::new([Widget::from(Text::new(title)), child])
                    .main_axis_size(MainAxisSize::Min)
                    .into()
            } else {
                child
            };
            groups[index].1.push(child);
        }
        Column::new(groups.into_iter().map(|(label, children)| {
            let title = ExcludeSemantics::new(Text::new(label.as_str()));
            let content = std::iter::once(Widget::from(title)).chain(children);
            Widget::from(
                Semantics::new(Column::new(content).main_axis_size(MainAxisSize::Min))
                    .container(true)
                    .explicit_child_nodes(true)
                    .role(Role::Group)
                    .label(label.as_str()),
            )
            .with_key(label.as_str())
        }))
        .main_axis_size(MainAxisSize::Min)
        .into()
    }
}
