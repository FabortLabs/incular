//! Screen-reader semantics as a transparent DOM tree over each canvas.
//!
//! Browsers expose only the DOM to assistive technology, and AccessKit has no
//! browser adapter. As in Flutter's web engine, semantics stay off until a
//! screen-reader user activates a visually hidden "Enable accessibility"
//! button. From then on every window's semantic nodes are mirrored as
//! elements with ARIA roles and states, laid out over the canvas at their
//! bounds:
//!
//! - Activating an element (a screen reader's click) and focusing one become
//!   semantic `Activate` and `Focus` actions.
//! - Key presses on an element are forwarded to the canvas, so focus-mode
//!   keyboard use reaches the application as ordinary keys. Enter and Space
//!   on an element that can be activated activate it instead.
//! - The application's focused node takes the page focus, unless a text
//!   field's editor holds it.
//!
//! The elements ignore the pointer; mouse and touch input keep reaching the
//! canvas.

use std::{cell::RefCell, collections::HashMap};

use incular_accessibility::{
    MobileAccessibilityNode, MobileAccessibilityUpdate, Role, SemanticAction, SemanticActionKind,
};
use incular_desktop::DesktopSemanticsEvent;
use wasm_bindgen::{JsCast as _, closure::Closure};
use winit::window::WindowId;

const HOST_STYLE: &str = "position:fixed;left:0;top:0;margin:0;padding:0;border:0;\
     pointer-events:none;overflow:hidden;z-index:1;";

const NODE_STYLE: &str = "position:fixed;display:block;margin:0;padding:0;border:0;\
     box-sizing:border-box;overflow:hidden;color:transparent;background:transparent;\
     outline:none;font-size:1px;line-height:1;white-space:pre;pointer-events:none;";

const PLACEHOLDER_STYLE: &str = "position:fixed;left:0;top:0;width:1px;height:1px;margin:0;\
     padding:0;border:0;overflow:hidden;opacity:0;pointer-events:none;";

/// Marks every element this module owns.
const OWNER_ATTRIBUTE: &str = "data-incular-semantics";

/// The text box role of a text input node, which only the innermost of
/// nested text input nodes keeps.
const TEXT_INPUT_ATTRIBUTE: &str = "data-incular-text-input";

thread_local! {
    static PAGE: RefCell<Page> = RefCell::new(Page::default());
    static EVENTS: RefCell<Vec<(WindowId, DesktopSemanticsEvent)>> = const { RefCell::new(Vec::new()) };
}

#[derive(Default)]
struct Page {
    windows: HashMap<WindowId, WindowSemantics>,
    placeholder: Option<web_sys::HtmlElement>,
    enabled: bool,
}

struct WindowSemantics {
    canvas: web_sys::HtmlCanvasElement,
    /// Created when semantics are enabled.
    host: Option<web_sys::HtmlElement>,
    nodes: HashMap<u64, NodeElement>,
}

struct NodeElement {
    element: web_sys::HtmlElement,
    actions: Vec<SemanticActionKind>,
    /// Kept so the element receives the page focus for this node.
    focus_target: bool,
}

/// Starts presenting semantics for a window's canvas once enabled.
pub(crate) fn register(window: WindowId, canvas: web_sys::HtmlCanvasElement) {
    let enabled = PAGE.with_borrow_mut(|page| {
        page.windows.insert(
            window,
            WindowSemantics {
                canvas,
                host: None,
                nodes: HashMap::new(),
            },
        );
        page.enabled
    });
    if enabled {
        push(window, DesktopSemanticsEvent::Enable);
    } else {
        ensure_placeholder();
    }
}

/// Removes a closed window's semantic elements.
pub(crate) fn unregister(window: WindowId) {
    let removed = PAGE.with_borrow_mut(|page| page.windows.remove(&window));
    if let Some(host) = removed.and_then(|semantics| semantics.host) {
        host.remove();
    }
}

pub(crate) fn take_events() -> Vec<(WindowId, DesktopSemanticsEvent)> {
    EVENTS.with_borrow_mut(std::mem::take)
}

/// Whether `target` is one of the semantic elements, whose focus the window
/// keeps as its own.
pub(crate) fn is_semantics_element(target: &web_sys::EventTarget) -> bool {
    target
        .dyn_ref::<web_sys::Element>()
        .is_some_and(|element| element.has_attribute(OWNER_ATTRIBUTE))
}

fn push(window: WindowId, event: DesktopSemanticsEvent) {
    EVENTS.with_borrow_mut(|events| events.push((window, event)));
    crate::wake_host();
}

fn document() -> Option<web_sys::Document> {
    web_sys::window().and_then(|window| window.document())
}

fn ensure_placeholder() {
    if PAGE.with_borrow(|page| page.placeholder.is_some() || page.enabled) {
        return;
    }
    let Some(document) = document() else {
        return;
    };
    let Some(button) = document
        .create_element("button")
        .ok()
        .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
    else {
        return;
    };
    let _ = button.set_attribute("style", PLACEHOLDER_STYLE);
    let _ = button.set_attribute("aria-label", "Enable accessibility");
    let _ = button.set_attribute(OWNER_ATTRIBUTE, "placeholder");
    // Keyboard users reach the application through the canvas; only screen
    // readers, which browse the document, should meet this button.
    let _ = button.set_attribute("tabindex", "-1");
    let enable = Closure::<dyn FnMut(web_sys::Event)>::new(|event: web_sys::Event| {
        event.prevent_default();
        enable_all();
    });
    let _ = button.add_event_listener_with_callback("click", enable.as_ref().unchecked_ref());
    enable.forget();
    if let Some(body) = document.body() {
        // First in the document, so a screen reader reaches it first.
        let _ = body.insert_before(&button, body.first_child().as_ref());
    }
    PAGE.with_borrow_mut(|page| page.placeholder = Some(button));
}

fn enable_all() {
    let (windows, placeholder) = PAGE.with_borrow_mut(|page| {
        page.enabled = true;
        (
            page.windows.keys().copied().collect::<Vec<_>>(),
            page.placeholder.take(),
        )
    });
    if let Some(placeholder) = placeholder {
        placeholder.remove();
    }
    for window in windows {
        push(window, DesktopSemanticsEvent::Enable);
    }
}

/// Mirrors one window's semantic update into its elements.
pub(crate) fn publish(window: WindowId, update: MobileAccessibilityUpdate) {
    let Some(document) = document() else {
        return;
    };
    let focus = PAGE.with_borrow_mut(|page| {
        let semantics = page.windows.get_mut(&window)?;
        if semantics.host.is_none() {
            semantics.host = create_host(&document);
        }
        let host = semantics.host.clone()?;
        place_host(&host, &semantics.canvas);
        for node in &update.nodes {
            apply_node(&document, window, semantics, node);
        }
        for native_id in &update.removed {
            if let Some(node) = semantics.nodes.remove(native_id) {
                node.element.remove();
            }
        }
        // Children are ordered after every changed node exists. A node no
        // longer listed by its parent leaves it.
        for node in &update.nodes {
            let Some(parent) = semantics.nodes.get(&node.native_id) else {
                continue;
            };
            let listed = node
                .children
                .iter()
                .filter_map(|child| semantics.nodes.get(child))
                .map(|child| child.element.clone())
                .collect::<Vec<_>>();
            let current = parent.element.children();
            for index in (0..current.length()).rev() {
                if let Some(element) = current.item(index)
                    && !listed
                        .iter()
                        .any(|child| child.unchecked_ref::<web_sys::Element>() == &element)
                {
                    element.remove();
                }
            }
            for child in &listed {
                let _ = parent.element.append_child(child);
            }
        }
        // Every top-level node, not only the tree's root, belongs to the
        // window, in the order the nodes appeared.
        let mut detached = semantics
            .nodes
            .iter()
            .filter(|(_, node)| node.element.parent_node().is_none())
            .map(|(native_id, node)| (*native_id, node.element.clone()))
            .collect::<Vec<_>>();
        detached.sort_unstable_by_key(|(native_id, _)| *native_id);
        for (_, element) in detached {
            let _ = host.append_child(&element);
        }
        announce_innermost_text_inputs(&host);
        let focused = semantics
            .nodes
            .values()
            .find(|node| node.focus_target)
            .map(|node| node.element.clone());
        Some((focused, semantics.canvas.clone()))
    });
    let Some((focused, canvas)) = focus else {
        return;
    };
    sync_focus(&document, focused, &canvas);
}

/// Wraps the host in a `transform`, which makes it the containing block of
/// its fixed-position descendants: node bounds are window coordinates.
fn create_host(document: &web_sys::Document) -> Option<web_sys::HtmlElement> {
    let host = document
        .create_element("div")
        .ok()?
        .dyn_into::<web_sys::HtmlElement>()
        .ok()?;
    let _ = host.set_attribute(OWNER_ATTRIBUTE, "host");
    let _ = host.set_attribute("style", HOST_STYLE);
    document.body()?.append_child(&host).ok()?;
    Some(host)
}

fn place_host(host: &web_sys::HtmlElement, canvas: &web_sys::HtmlCanvasElement) {
    let rect = canvas.get_bounding_client_rect();
    let style = host.style();
    let _ = style.set_property(
        "transform",
        &format!("translate({}px,{}px)", rect.left(), rect.top()),
    );
    let _ = style.set_property("width", &format!("{}px", rect.width()));
    let _ = style.set_property("height", &format!("{}px", rect.height()));
}

fn apply_node(
    document: &web_sys::Document,
    window: WindowId,
    semantics: &mut WindowSemantics,
    node: &MobileAccessibilityNode,
) {
    let semantic = &node.semantic;
    let (tag, role) = element_kind(semantic.role, semantic.label.is_some());
    let existing = semantics
        .nodes
        .get(&node.native_id)
        .filter(|existing| existing.element.tag_name().eq_ignore_ascii_case(tag))
        .map(|existing| existing.element.clone());
    let element = match existing {
        Some(element) => element,
        None => {
            let Some(element) = create_node_element(document, window, node.native_id, tag) else {
                return;
            };
            // A node whose kind changed keeps its place and children.
            if let Some(old) = semantics.nodes.get(&node.native_id) {
                let _ = old.element.replace_with_with_node_1(&element);
                while let Some(child) = old.element.first_child() {
                    let _ = element.append_child(&child);
                }
            }
            element
        }
    };
    let bounds = semantic.bounds;
    let _ = element.set_attribute(
        "style",
        &format!(
            "{NODE_STYLE}left:{}px;top:{}px;width:{}px;height:{}px;",
            bounds.origin.x,
            bounds.origin.y,
            bounds.size.width.max(0.0),
            bounds.size.height.max(0.0)
        ),
    );
    set_or_remove(&element, "role", role);
    let state = &semantic.state;
    let leaf_text = node.children.is_empty() && matches!(semantic.role, Role::Text | Role::Heading);
    let text = if leaf_text {
        semantic.label.clone()
    } else if is_text_input(semantic.role) {
        semantic.value.clone()
    } else {
        None
    };
    // Text content would replace child elements, so only leaves carry it.
    if node.children.is_empty() {
        let current = element.text_content().unwrap_or_default();
        let next = text.clone().unwrap_or_default();
        if current != next {
            element.set_text_content(Some(&next));
        }
    }
    set_or_remove(
        &element,
        "aria-label",
        semantic.label.as_deref().filter(|_| !leaf_text),
    );
    set_or_remove(
        &element,
        "aria-description",
        semantic.description.as_deref(),
    );
    let interactive = state.focusable
        || semantic.actions.contains(&SemanticActionKind::Focus)
        || semantic.actions.contains(&SemanticActionKind::Activate);
    set_flag(
        &element,
        "aria-disabled",
        (interactive || is_text_input(semantic.role)) && !state.enabled,
    );
    set_or_remove(
        &element,
        TEXT_INPUT_ATTRIBUTE,
        role.filter(|_| is_text_input(semantic.role)),
    );
    set_or_remove(
        &element,
        "aria-checked",
        state.checked.map(|checked| checked.to_string()).as_deref(),
    );
    set_or_remove(
        &element,
        "aria-expanded",
        state
            .expanded
            .map(|expanded| expanded.to_string())
            .as_deref(),
    );
    set_flag(
        &element,
        "aria-selected",
        state.selected && matches!(semantic.role, Role::Tab | Role::ListItem),
    );
    set_flag(
        &element,
        "aria-readonly",
        state.read_only && is_text_input(semantic.role),
    );
    set_flag(&element, "aria-multiline", state.multiline);
    set_flag(&element, "aria-required", state.required);
    set_flag(&element, "aria-invalid", state.invalid);
    set_flag(&element, "aria-busy", state.busy);
    let number = |value: Option<f64>| value.map(|value| value.to_string());
    set_or_remove(
        &element,
        "aria-valuenow",
        number(state.numeric_value).as_deref(),
    );
    set_or_remove(
        &element,
        "aria-valuemin",
        number(state.numeric_min).as_deref(),
    );
    set_or_remove(
        &element,
        "aria-valuemax",
        number(state.numeric_max).as_deref(),
    );
    set_or_remove(
        &element,
        "aria-valuetext",
        semantic.value.as_deref().filter(|_| {
            matches!(
                semantic.role,
                Role::Slider | Role::ProgressBar | Role::Meter
            )
        }),
    );
    set_or_remove(
        &element,
        "aria-level",
        state
            .heading_level
            .map(|level| level.to_string())
            .as_deref(),
    );
    set_or_remove(
        &element,
        "aria-posinset",
        state
            .item_index
            .map(|index| (index + 1).to_string())
            .as_deref(),
    );
    set_or_remove(
        &element,
        "aria-setsize",
        state.set_size.map(|size| size.to_string()).as_deref(),
    );
    set_or_remove(&element, "tabindex", interactive.then_some("-1"));
    semantics.nodes.insert(
        node.native_id,
        NodeElement {
            element,
            actions: semantic.actions.clone(),
            focus_target: state.focused && !state.editable,
        },
    );
}

fn create_node_element(
    document: &web_sys::Document,
    window: WindowId,
    native_id: u64,
    tag: &str,
) -> Option<web_sys::HtmlElement> {
    let element = document
        .create_element(tag)
        .ok()?
        .dyn_into::<web_sys::HtmlElement>()
        .ok()?;
    let _ = element.set_attribute(OWNER_ATTRIBUTE, &native_id.to_string());
    let on_click = Closure::<dyn FnMut(web_sys::Event)>::new(move |event: web_sys::Event| {
        event.prevent_default();
        event.stop_propagation();
        // Activating a text field focuses it.
        if !request(window, native_id, SemanticAction::Activate) {
            request(window, native_id, SemanticAction::Focus);
        }
    });
    let _ = element.add_event_listener_with_callback("click", on_click.as_ref().unchecked_ref());
    on_click.forget();
    let on_focus = Closure::<dyn FnMut(web_sys::Event)>::new(move |event: web_sys::Event| {
        // Only the element itself, not focus bubbling from a descendant.
        if event.target() == event.current_target() {
            request(window, native_id, SemanticAction::Focus);
        }
    });
    let _ = element.add_event_listener_with_callback("focus", on_focus.as_ref().unchecked_ref());
    on_focus.forget();
    for kind in ["keydown", "keyup"] {
        let forward = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(
            move |event: web_sys::KeyboardEvent| forward_key(window, native_id, &event),
        );
        let _ = element.add_event_listener_with_callback(kind, forward.as_ref().unchecked_ref());
        forward.forget();
    }
    Some(element)
}

/// A text field's outer semantic nodes wrap its editable one. Only that
/// innermost node is announced as a text box; the others are its labelled
/// groups.
fn announce_innermost_text_inputs(host: &web_sys::HtmlElement) {
    let selector = format!("[{TEXT_INPUT_ATTRIBUTE}]");
    let Ok(inputs) = host.query_selector_all(&selector) else {
        return;
    };
    for index in 0..inputs.length() {
        let Some(input) = inputs
            .item(index)
            .and_then(|node| node.dyn_into::<web_sys::Element>().ok())
        else {
            continue;
        };
        let wraps_input = input.query_selector(&selector).ok().flatten().is_some();
        let role = if wraps_input {
            Some("group".to_owned())
        } else {
            input.get_attribute(TEXT_INPUT_ATTRIBUTE)
        };
        if let Some(role) = role
            && input.get_attribute("role").as_deref() != Some(role.as_str())
        {
            let _ = input.set_attribute("role", &role);
        }
    }
}

/// Requests `action` on the node, or on its first descendant that offers it
/// when the node does not (a text field's wrapper, for instance). Returns
/// whether a node offered it.
fn request(window: WindowId, native_id: u64, action: SemanticAction) -> bool {
    let target = PAGE.with_borrow(|page| {
        let semantics = page.windows.get(&window)?;
        let offers = |native_id: u64| {
            semantics
                .nodes
                .get(&native_id)
                .is_some_and(|node| node.actions.contains(&action.kind()))
        };
        if offers(native_id) {
            return Some(native_id);
        }
        let descendants = semantics
            .nodes
            .get(&native_id)?
            .element
            .query_selector_all(&format!("[{OWNER_ATTRIBUTE}]"))
            .ok()?;
        (0..descendants.length())
            .filter_map(|index| descendants.item(index))
            .filter_map(|node| node.dyn_into::<web_sys::Element>().ok())
            .filter_map(|element| element.get_attribute(OWNER_ATTRIBUTE)?.parse().ok())
            .find(|native_id| offers(*native_id))
    });
    if let Some(native_id) = target {
        push(window, DesktopSemanticsEvent::Action { native_id, action });
    }
    target.is_some()
}

/// Hands a key press on a semantic element to the window's canvas, where
/// Winit and the application handle it.
fn forward_key(window: WindowId, native_id: u64, event: &web_sys::KeyboardEvent) {
    if event.target() != event.current_target() {
        return;
    }
    let Some((canvas, activates)) = PAGE.with_borrow(|page| {
        let semantics = page.windows.get(&window)?;
        let activates = semantics
            .nodes
            .get(&native_id)
            .is_some_and(|node| node.actions.contains(&SemanticActionKind::Activate));
        Some((semantics.canvas.clone(), activates))
    }) else {
        return;
    };
    event.prevent_default();
    event.stop_propagation();
    // Enter and Space activate, as on a native button, whether or not the
    // application's own focus is on this node.
    let plain = !(event.ctrl_key() || event.alt_key() || event.meta_key() || event.shift_key());
    if activates && plain && matches!(event.key().as_str(), "Enter" | " ") {
        if event.type_() == "keydown" && !event.repeat() {
            request(window, native_id, SemanticAction::Activate);
        }
        return;
    }
    crate::text_input::dispatch_key_copy(event, &canvas);
}

/// Gives the page focus to the application's focused node, so a screen
/// reader follows it. A text field's editor keeps the focus it took, and the
/// canvas takes it back when no node is focused.
fn sync_focus(
    document: &web_sys::Document,
    focused: Option<web_sys::HtmlElement>,
    canvas: &web_sys::HtmlCanvasElement,
) {
    let active = document.active_element();
    if active
        .as_ref()
        .is_some_and(|active| crate::text_input::is_editor(active.unchecked_ref()))
    {
        return;
    }
    let options = web_sys::FocusOptions::new();
    options.set_prevent_scroll(true);
    match focused {
        Some(element) => {
            if active.as_ref() != Some(element.unchecked_ref()) {
                let _ = element.focus_with_options(&options);
            }
        }
        None => {
            let on_semantics = active
                .as_ref()
                .is_some_and(|active| is_semantics_element(active.unchecked_ref()));
            // Removing a focused element leaves the focus on the body.
            let lost = active.is_none()
                || document
                    .body()
                    .is_some_and(|body| active.as_ref() == Some(body.unchecked_ref()));
            if on_semantics || lost {
                let _ = canvas.focus_with_options(&options);
            }
        }
    }
}

fn element_kind(role: Role, labelled: bool) -> (&'static str, Option<&'static str>) {
    match role {
        Role::Button => ("div", Some("button")),
        Role::Text => ("span", None),
        Role::TextField => ("div", Some("textbox")),
        Role::TextArea => ("div", Some("textbox")),
        Role::SearchField => ("div", Some("searchbox")),
        Role::List => ("div", Some("list")),
        Role::ListItem => ("div", Some("listitem")),
        Role::ScrollView | Role::Group => ("div", Some("group")),
        // A label on an anonymous container is only announced on a group.
        Role::GenericContainer => ("div", labelled.then_some("group")),
        Role::Checkbox => ("div", Some("checkbox")),
        Role::Radio => ("div", Some("radio")),
        Role::Switch => ("div", Some("switch")),
        Role::Slider => ("div", Some("slider")),
        Role::Menu => ("div", Some("menu")),
        Role::MenuItem => ("div", Some("menuitem")),
        Role::Dialog => ("div", Some("dialog")),
        Role::Image => ("div", Some("img")),
        Role::Heading => ("div", Some("heading")),
        Role::Link => ("div", Some("link")),
        Role::Tab => ("div", Some("tab")),
        Role::TabList => ("div", Some("tablist")),
        Role::TabPanel => ("div", Some("tabpanel")),
        Role::ProgressBar => ("div", Some("progressbar")),
        Role::Meter => ("div", Some("meter")),
    }
}

fn is_text_input(role: Role) -> bool {
    matches!(role, Role::TextField | Role::TextArea | Role::SearchField)
}

fn set_or_remove(element: &web_sys::HtmlElement, name: &str, value: Option<&str>) {
    match value {
        Some(value) => {
            if element.get_attribute(name).as_deref() != Some(value) {
                let _ = element.set_attribute(name, value);
            }
        }
        None => {
            let _ = element.remove_attribute(name);
        }
    }
}

fn set_flag(element: &web_sys::HtmlElement, name: &str, value: bool) {
    set_or_remove(element, name, value.then_some("true"));
}
