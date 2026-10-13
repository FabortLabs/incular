/// Implements `From<$widget> for Widget` for `WidgetKind` variants that move
/// same-named fields one-to-one. Renames and transformations keep a hand-written
/// impl so the mapping stays visible.
macro_rules! widget_from_kind {
    ($widget:ident => $kind:ident { $($field:ident),+ $(,)? }) => {
        impl From<$widget> for Widget {
            fn from(value: $widget) -> Self {
                Widget::from_kind(WidgetKind::$kind {
                    $($field: value.$field,)+
                })
            }
        }
    };
}

pub(crate) use widget_from_kind;
