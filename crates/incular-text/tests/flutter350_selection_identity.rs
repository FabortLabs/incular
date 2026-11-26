use std::{cell::RefCell, collections::HashSet, rc::Rc};

use TextAffinity::{Downstream, Upstream};
use incular_text::{FlutterTextSelection, TextAffinity, TextEditingController, TextSelection};

#[test]
fn every_invalid_flutter_selection_is_one_equivalence_class() {
    let first = FlutterTextSelection {
        is_directional: true,
        ..FlutterTextSelection::new(-1, 0)
    };
    let second = FlutterTextSelection {
        affinity: Upstream,
        ..FlutterTextSelection::new(123, -1)
    };
    assert!(!first.is_valid() && !second.is_valid());
    assert_eq!(first, second);
    assert_eq!(HashSet::from([first, second]).len(), 1);
    assert_ne!(first, FlutterTextSelection::new(0, 0));
}

#[test]
fn noncollapsed_affinity_does_not_change_equality_or_hash() {
    let downstream = FlutterTextSelection::new(1, 2);
    let upstream = FlutterTextSelection {
        affinity: Upstream,
        ..downstream
    };
    assert_eq!(downstream, upstream);
    assert_eq!(HashSet::from([downstream, upstream]).len(), 1);
    assert_ne!(downstream, FlutterTextSelection::new(2, 1));
    assert_ne!(
        downstream,
        FlutterTextSelection {
            is_directional: true,
            ..upstream
        }
    );
    let collapsed = FlutterTextSelection::new(2, 2);
    assert_ne!(
        collapsed,
        FlutterTextSelection {
            affinity: Upstream,
            ..collapsed
        }
    );
}

#[test]
fn collapsed_affinity_notifies_before_observers_read_the_visual_caret() {
    let controller = TextEditingController::with_text("aب");
    controller.set_selection(TextSelection::collapsed(1));
    let observed = Rc::new(RefCell::new(Vec::new()));
    let observer = controller.observe({
        let controller = controller.clone();
        let observed = observed.clone();
        move |value| {
            observed
                .borrow_mut()
                .push((value.selection, controller.caret_affinity()))
        }
    });
    let before = controller.revisions();
    controller.set_selection_with_affinity(TextSelection::collapsed(1), Upstream);
    assert_eq!(observed.take(), [(TextSelection::collapsed(1), Upstream)]);
    assert_eq!(controller.revisions(), (before.0, before.1 + 1));

    controller.set_selection_with_affinity(TextSelection::new(0, 1), Upstream);
    assert_eq!(observed.take(), [(TextSelection::new(0, 1), Upstream)]);
    let before = controller.revisions();
    controller.set_selection_with_affinity(TextSelection::new(0, 1), Downstream);
    assert_eq!(controller.revisions(), before);
    assert!(observed.borrow().is_empty());

    controller.apply_autofill_value(controller.value(), true, Downstream, false);
    assert_eq!(controller.revisions(), before);
    assert!(observed.borrow().is_empty());
    drop(observer);
}
