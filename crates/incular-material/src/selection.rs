//! Material adapters for retained selection controls and sliders.

#[path = "selection/checkbox_radio.rs"]
mod checkbox_radio;
#[path = "selection/slider_range.rs"]
mod slider_range;
#[path = "selection/switch.rs"]
mod switch;
#[path = "selection/vocabulary.rs"]
mod vocabulary;

pub use checkbox_radio::*;
pub use slider_range::*;
pub use switch::*;
pub use vocabulary::*;

fn resolved_control_theme(
    context: &incular_widgets::BuildContext<'_>,
) -> (
    incular_controls::ControlTheme,
    Option<std::rc::Rc<crate::material_theme::SelectionControlThemes>>,
) {
    let mut controls = incular_controls::current_control_theme(context);
    // Flutter's Material 3 defaults: `_SwitchConfigM3` (a 52x32 track, a
    // 16 px thumb that grows to 24 px when on), `_CheckboxDefaultsM3`'s
    // 2 px corner radius and `_SliderDefaultsM3Year2023` (a 4 px track,
    // `RoundSliderThumbShape`'s 10 px radius).
    controls.switch.width = 52.0;
    controls.switch.height = 32.0;
    controls.switch.thumb_size = 16.0;
    controls.switch.active_thumb_size = 24.0;
    controls.checkbox.radius = 2.0;
    controls.slider.track_height = 4.0;
    controls.slider.thumb_size = 20.0;
    let selection = context.depend_on_shared::<crate::material_theme::SelectionControlThemes>();
    if let Some(selection) = selection.as_ref() {
        if let Some(size) = selection.checkbox_theme.icon_size {
            controls.checkbox.indicator_size = size.max(1.0);
        }
        if let Some(size) = selection.radio_theme.icon_size {
            controls.radio.indicator_size = size.max(1.0);
        }
        if let Some(size) = selection.switch_theme.minimum_size {
            controls.switch.width = size.width.max(1.0);
            controls.switch.height = size.height.max(1.0);
        }
        if let Some(size) = selection.switch_theme.thumb_size {
            controls.switch.thumb_size = size.max(1.0);
        }
        if let Some(height) = selection.slider_theme.track_height {
            controls.slider.track_height = height.max(1.0);
        }
        if let Some(size) = selection.slider_theme.thumb_size {
            controls.slider.thumb_size = size.max(1.0);
        }
    }
    (controls, selection)
}
