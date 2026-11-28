//! Focus entry and exit on explicit host view-focus events.

use incular_config::{Constraints, TextDirection};
use incular_core::{Size, time::Instant};
use incular_platform::{
    WindowEvent, WindowFocusDirection as Direction, WindowFocusEvent, WindowId,
};
use incular_runtime::Application;
use incular_widgets::{Column, Directionality, Focus, FocusNode, SizedBox, Widget};

struct Host {
    app: Application,
    view: WindowId,
    nodes: Vec<FocusNode>,
}

impl Host {
    fn new(count: usize) -> Self {
        let nodes = (0..count).map(|_| FocusNode::new()).collect::<Vec<_>>();
        let children = nodes
            .iter()
            .map(|node| Widget::from(Focus::new(SizedBox::new().height(50.)).node(node.clone())))
            .collect::<Vec<_>>();
        let root: Widget = Directionality::new(TextDirection::Ltr, Column::new(children)).into();
        let app = Application::new(move |_| root.clone()).unwrap();
        let view = app.primary_window();
        Self { app, view, nodes }
    }

    fn frame(&mut self) {
        self.app
            .run_window_frame_at(
                self.view,
                Constraints::tight(Size::new(400., 200.)),
                Instant::now(),
            )
            .unwrap()
            .unwrap();
    }

    fn notify(&mut self, focused: bool, direction: Direction) {
        self.app
            .handle_window_event(WindowEvent::focus(WindowFocusEvent {
                window_id: self.view,
                focused,
                direction,
            }));
        self.frame();
    }

    fn focused(&self) -> Option<usize> {
        let mut current = self
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node.has_primary_focus());
        let result = current.next().map(|(index, _)| index);
        assert!(current.next().is_none(), "one primary focus per view");
        result
    }
}

#[test]
fn exiting_the_view_retires_focus_and_backward_entry_restores_its_only_child() {
    let mut host = Host::new(1);
    host.frame();
    host.nodes[0].request_focus();
    host.frame();
    assert_eq!(host.focused(), Some(0));

    host.notify(false, Direction::Forward);
    assert_eq!(host.focused(), None);
    host.notify(true, Direction::Backward);
    assert_eq!(host.focused(), Some(0));
}

#[test]
fn undefined_direction_keeps_a_newly_requested_child_and_can_restore_history() {
    let mut host = Host::new(2);
    host.frame();
    host.notify(false, Direction::Undefined);
    assert_eq!(host.focused(), None);

    host.nodes[1].request_focus();
    host.notify(true, Direction::Undefined);
    assert_eq!(host.focused(), Some(1));

    host.notify(false, Direction::Undefined);
    assert_eq!(host.focused(), None);
    host.notify(true, Direction::Undefined);
    assert_eq!(host.focused(), Some(1));
}

#[test]
fn explicit_traversal_enters_at_the_first_or_last_child_despite_a_pending_request() {
    let mut host = Host::new(2);
    host.frame();
    host.notify(false, Direction::Undefined);
    host.nodes[1].request_focus();
    host.notify(true, Direction::Forward);
    assert_eq!(host.focused(), Some(0));

    host.notify(false, Direction::Undefined);
    host.nodes[0].request_focus();
    host.notify(true, Direction::Backward);
    assert_eq!(host.focused(), Some(1));
}

#[test]
fn directional_entry_can_arrive_before_the_first_layout() {
    let mut host = Host::new(2);
    host.nodes[0].request_focus();
    host.notify(true, Direction::Backward);
    assert_eq!(host.focused(), Some(1));
}

#[test]
fn a_widget_request_after_the_directional_event_takes_precedence() {
    let mut host = Host::new(2);
    host.frame();
    host.notify(false, Direction::Undefined);
    host.app
        .handle_window_event(WindowEvent::focus(WindowFocusEvent {
            window_id: host.view,
            focused: true,
            direction: Direction::Forward,
        }));
    host.nodes[1].request_focus();
    host.frame();
    assert_eq!(host.focused(), Some(1));
}

#[test]
fn view_focus_restores_the_editor_range_in_a_fresh_text_input_session() {
    use incular_platform::TextInputCommand;
    use incular_text::{TextEditingController, TextSelection};
    use incular_widgets::EditableText;

    let focus = FocusNode::new();
    let controller = TextEditingController::with_text("abcdef");
    let root = Widget::from(
        EditableText::new(controller.clone())
            .focus_node(focus.clone())
            .text_direction(TextDirection::Ltr),
    );
    let app = Application::new(move |_| root.clone()).unwrap();
    let view = app.primary_window();
    let mut host = Host {
        app,
        view,
        nodes: vec![focus.clone()],
    };
    host.frame();
    focus.request_focus();
    host.frame();
    let selection = TextSelection::new(4, 1);
    controller.set_selection(selection);
    host.frame();

    let initial_client = host
        .app
        .take_window_text_input_commands(view)
        .into_iter()
        .find_map(|command| match command {
            TextInputCommand::SetClient { configuration, .. } => Some(configuration.client),
            _ => None,
        })
        .expect("initial native text-input client");

    host.notify(false, Direction::Undefined);
    assert!(!focus.has_primary_focus());
    assert!(
        host.app
            .take_window_text_input_commands(view)
            .iter()
            .any(|command| {
                matches!(command, TextInputCommand::Hide { client } if *client == initial_client)
            })
    );
    host.notify(true, Direction::Undefined);
    assert!(focus.has_primary_focus());
    assert_eq!(controller.selection(), selection);
    assert!(host.app.take_window_text_input_commands(view).iter().any(|command| {
        matches!(command, TextInputCommand::SetClient { configuration, .. } if configuration.client != initial_client)
    }));
}
