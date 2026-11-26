use incular_config::{Clip, Constraints, TextDirection};
use incular_core::{Color, InputEvent, Offset, Rect, Size};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_text::{TextEditingController, TextSelection};
use incular_widgets::{
    BorderSide, BoxShape, EditableText, FocusNode, OvalBorder, PhysicalModel, PhysicalShape,
    ShapeBorder, Widget,
};

#[test]
fn changing_a_physical_clip_does_not_unfocus_or_remount_its_editor() {
    let size = Size::new(120., 40.);
    let bounds = Constraints::tight(size);
    for custom in [false, true] {
        let controller = TextEditingController::with_text("A");
        let node = FocusNode::new();
        let build = |mode| -> Widget {
            let child = EditableText::new(controller.clone())
                .focus_node(node.clone())
                .text_direction(TextDirection::Ltr);
            if custom {
                let rect = Rect::from_origin_size(Offset::ZERO, size);
                let outline = OvalBorder::new(BorderSide::NONE).outer_path(rect);
                PhysicalShape::new(outline, Color::WHITE, child)
                    .clip_behavior(mode)
                    .into()
            } else {
                PhysicalModel::new(Color::WHITE, child)
                    .shape(BoxShape::Circle)
                    .clip_behavior(mode)
                    .into()
            }
        };

        let mut runtime = Runtime::new(build(Clip::None)).unwrap();
        runtime.set_text_shortcut_platform(TextShortcutPlatform::Linux);
        runtime.run_frame(bounds).unwrap();
        node.request_focus();
        runtime.run_frame(bounds).unwrap();
        assert!(node.has_primary_focus());
        let field = runtime
            .focused_element()
            .expect("the nested editor is focused");
        controller.set_selection(TextSelection::collapsed(1));

        for (mode, input, expected) in [
            (Clip::AntiAlias, "B", "AB"),
            (Clip::AntiAliasWithSaveLayer, "C", "ABC"),
            (Clip::None, "D", "ABCD"),
        ] {
            runtime
                .schedule_update(runtime.tree().root().unwrap(), build(mode))
                .unwrap();
            runtime.run_frame(bounds).unwrap();
            assert_eq!(runtime.focused_element(), Some(field), "{custom} {mode:?}");
            assert!(node.has_primary_focus(), "{custom} {mode:?}");
            let _ = runtime.handle_input(InputEvent::Text(input.into()));
            assert_eq!(controller.text(), expected, "{custom} {mode:?}");
        }
    }
}
