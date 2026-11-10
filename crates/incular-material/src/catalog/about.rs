//! Flutter's `AboutDialog`, `AboutListTile`, `LicensePage` and
//! `LicenseRegistry` (`material/about.dart`, `foundation/licenses.dart`).

use std::{cell::RefCell, rc::Rc};

use incular_config::{CrossAxisAlignment, EdgeInsets, MainAxisSize};
use incular_widgets::{Column, Container, ListView, Row, SizedBox, Text, Widget};

use super::theme_of;
use crate::{AlertDialog, ExpansionTile, ListTile, TextButton};

/// Flutter's `LicenseEntry`: license text for one or more packages.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LicenseEntry {
    pub packages: Vec<String>,
    /// Paragraphs of license text.
    pub paragraphs: Vec<String>,
}

impl LicenseEntry {
    /// Flutter's `LicenseEntryWithLineBreaks`: paragraphs split at blank
    /// lines.
    #[must_use]
    pub fn with_line_breaks(
        packages: impl IntoIterator<Item = impl Into<String>>,
        text: &str,
    ) -> Self {
        Self {
            packages: packages.into_iter().map(Into::into).collect(),
            paragraphs: text
                .split("\n\n")
                .map(|paragraph| paragraph.trim().to_owned())
                .filter(|paragraph| !paragraph.is_empty())
                .collect(),
        }
    }
}

thread_local! {
    static LICENSES: RefCell<Vec<LicenseEntry>> = const { RefCell::new(Vec::new()) };
}

/// Flutter's `LicenseRegistry`: licenses the application ships with, for the
/// [`LicensePage`].
pub struct LicenseRegistry;

impl LicenseRegistry {
    /// `LicenseRegistry.addLicense`.
    pub fn add_license(entry: LicenseEntry) {
        LICENSES.with(|licenses| licenses.borrow_mut().push(entry));
    }

    /// `LicenseRegistry.licenses`.
    #[must_use]
    pub fn licenses() -> Vec<LicenseEntry> {
        LICENSES.with(|licenses| licenses.borrow().clone())
    }

    /// The packages with licenses, in first-registered order.
    #[must_use]
    pub fn packages() -> Vec<String> {
        let mut packages = Vec::<String>::new();
        for entry in Self::licenses() {
            for package in entry.packages {
                if !packages.contains(&package) {
                    packages.push(package);
                }
            }
        }
        packages
    }

    /// `LicenseRegistry.reset`.
    pub fn reset() {
        LICENSES.with(|licenses| licenses.borrow_mut().clear());
    }
}

/// Flutter's `AboutDialog`: the application's icon, name, version and
/// legalese, with buttons to view licenses and to close.
#[derive(Clone)]
pub struct AboutDialog {
    pub application_name: String,
    pub application_version: Option<String>,
    pub application_icon: Option<Widget>,
    pub application_legalese: Option<String>,
    pub children: Vec<Widget>,
    pub on_view_licenses: Option<Rc<dyn Fn()>>,
    pub on_close: Option<Rc<dyn Fn()>>,
}

impl AboutDialog {
    #[must_use]
    pub fn new(application_name: impl Into<String>) -> Self {
        Self {
            application_name: application_name.into(),
            application_version: None,
            application_icon: None,
            application_legalese: None,
            children: Vec::new(),
            on_view_licenses: None,
            on_close: None,
        }
    }

    #[must_use]
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.application_version = Some(version.into());
        self
    }

    #[must_use]
    pub fn icon(mut self, icon: impl Into<Widget>) -> Self {
        self.application_icon = Some(icon.into());
        self
    }

    #[must_use]
    pub fn legalese(mut self, legalese: impl Into<String>) -> Self {
        self.application_legalese = Some(legalese.into());
        self
    }

    #[must_use]
    pub fn children(mut self, children: impl IntoIterator<Item = impl Into<Widget>>) -> Self {
        self.children = children.into_iter().map(Into::into).collect();
        self
    }

    #[must_use]
    pub fn on_view_licenses(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_view_licenses = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn on_close(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_close = Some(Rc::new(callback));
        self
    }
}

impl From<AboutDialog> for Widget {
    fn from(dialog: AboutDialog) -> Self {
        incular_widgets::Builder::new(move |context| {
            let theme = theme_of(context);
            let text = &theme.core().text_theme;
            let mut heading = vec![Widget::from(
                Text::new(dialog.application_name.clone()).style(text.headline_small.clone()),
            )];
            if let Some(version) = &dialog.application_version {
                heading.push(
                    Text::new(version.clone())
                        .style(text.body_medium.clone())
                        .into(),
                );
            }
            if let Some(legalese) = &dialog.application_legalese {
                heading.push(SizedBox::new().height(18.).into());
                heading.push(
                    Text::new(legalese.clone())
                        .style(text.body_small.clone())
                        .into(),
                );
            }
            let heading: Widget = Column::new(heading)
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .into();
            let mut header = Vec::new();
            if let Some(icon) = &dialog.application_icon {
                header.push(icon.clone());
                header.push(SizedBox::new().width(24.).into());
            }
            header.push(incular_widgets::Expanded::new(heading).into());
            let mut body = vec![Widget::from(
                Row::new(header).cross_axis_alignment(CrossAxisAlignment::Start),
            )];
            body.extend(dialog.children.iter().cloned());
            let view = dialog.on_view_licenses.clone();
            let close = dialog.on_close.clone();
            AlertDialog::new()
                .content(
                    Column::new(body)
                        .main_axis_size(MainAxisSize::Min)
                        .cross_axis_alignment(CrossAxisAlignment::Stretch),
                )
                .actions([
                    Widget::from(TextButton::new("View licenses").on_pressed(move || {
                        if let Some(view) = &view {
                            view();
                        }
                    })),
                    Widget::from(TextButton::new("Close").on_pressed(move || {
                        if let Some(close) = &close {
                            close();
                        }
                    })),
                ])
                .into()
        })
        .into()
    }
}

/// Flutter's `AboutListTile`: a list tile that shows the about dialog.
#[derive(Clone)]
pub struct AboutListTile {
    pub icon: Option<Widget>,
    pub child: Option<Widget>,
    pub application_name: String,
    pub on_tap: Option<Rc<dyn Fn()>>,
}

impl AboutListTile {
    #[must_use]
    pub fn new(application_name: impl Into<String>) -> Self {
        Self {
            icon: None,
            child: None,
            application_name: application_name.into(),
            on_tap: None,
        }
    }

    #[must_use]
    pub fn icon(mut self, icon: impl Into<Widget>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Called to show the about dialog.
    #[must_use]
    pub fn on_tap(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_tap = Some(Rc::new(callback));
        self
    }
}

impl From<AboutListTile> for Widget {
    fn from(tile: AboutListTile) -> Self {
        let title = tile
            .child
            .clone()
            .unwrap_or_else(|| Text::new(format!("About {}", tile.application_name)).into());
        let mut list_tile = ListTile::new(title);
        if let Some(icon) = tile.icon {
            list_tile = list_tile.leading(icon);
        }
        match tile.on_tap {
            Some(on_tap) => incular_widgets::GestureDetector::new(list_tile)
                .on_tap(move || on_tap())
                .into(),
            None => list_tile.into(),
        }
    }
}

/// Flutter's `LicensePage`: the application header over every registered
/// package, each expanding to its licenses.
#[derive(Clone)]
pub struct LicensePage {
    pub application_name: Option<String>,
    pub application_version: Option<String>,
    pub application_icon: Option<Widget>,
    pub application_legalese: Option<String>,
}

impl LicensePage {
    #[must_use]
    pub fn new() -> Self {
        Self {
            application_name: None,
            application_version: None,
            application_icon: None,
            application_legalese: None,
        }
    }

    #[must_use]
    pub fn application_name(mut self, name: impl Into<String>) -> Self {
        self.application_name = Some(name.into());
        self
    }

    #[must_use]
    pub fn application_version(mut self, version: impl Into<String>) -> Self {
        self.application_version = Some(version.into());
        self
    }

    #[must_use]
    pub fn application_icon(mut self, icon: impl Into<Widget>) -> Self {
        self.application_icon = Some(icon.into());
        self
    }

    #[must_use]
    pub fn application_legalese(mut self, legalese: impl Into<String>) -> Self {
        self.application_legalese = Some(legalese.into());
        self
    }
}

impl Default for LicensePage {
    fn default() -> Self {
        Self::new()
    }
}

impl From<LicensePage> for Widget {
    fn from(page: LicensePage) -> Self {
        incular_widgets::Builder::new(move |context| {
            let theme = theme_of(context);
            let text = &theme.core().text_theme;
            let scheme = &theme.core().color_scheme;
            // `_AboutProgram`.
            let mut header: Vec<Widget> = Vec::new();
            if let Some(name) = &page.application_name {
                header.push(
                    Text::new(name.clone())
                        .style(text.headline_small.clone())
                        .into(),
                );
            }
            if let Some(icon) = &page.application_icon {
                header.push(icon.clone());
            }
            if let Some(version) = &page.application_version {
                header.push(
                    Text::new(version.clone())
                        .style(text.body_medium.clone())
                        .into(),
                );
            }
            if let Some(legalese) = &page.application_legalese {
                header.push(
                    Text::new(legalese.clone())
                        .style(text.body_small.clone())
                        .into(),
                );
            }
            header.push(
                Text::new("Powered by Incular")
                    .style(text.body_medium.clone())
                    .into(),
            );
            let mut items: Vec<Widget> = vec![
                Container::new()
                    .padding(EdgeInsets::symmetric(24., 24.))
                    .child(
                        Column::new(header)
                            .main_axis_size(MainAxisSize::Min)
                            .spacing(12.),
                    )
                    .into(),
            ];
            let licenses = LicenseRegistry::licenses();
            for package in LicenseRegistry::packages() {
                let paragraphs = licenses
                    .iter()
                    .filter(|entry| entry.packages.contains(&package))
                    .flat_map(|entry| entry.paragraphs.iter().cloned())
                    .collect::<Vec<_>>();
                let count = licenses
                    .iter()
                    .filter(|entry| entry.packages.contains(&package))
                    .count();
                let body = paragraphs.into_iter().map(|paragraph| {
                    Widget::from(
                        Container::new()
                            .padding(EdgeInsets::only(16., 8., 16., 0.))
                            .child(
                                Text::new(paragraph)
                                    .style(text.body_medium.clone())
                                    .color(scheme.on_surface),
                            ),
                    )
                });
                items.push(
                    ExpansionTile::text(package)
                        .subtitle(Text::new(if count == 1 {
                            "1 license.".to_owned()
                        } else {
                            format!("{count} licenses.")
                        }))
                        .children(body.collect::<Vec<_>>())
                        .expanded_cross_axis_alignment(CrossAxisAlignment::Start)
                        .into(),
                );
            }
            ListView::new(items).into()
        })
        .into()
    }
}
