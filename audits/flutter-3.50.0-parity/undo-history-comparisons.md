# Undo history: baseline timing and grouped edits

Pinned Flutter `ab2912035e`, Incular `c647cb85`. The existing strict
keyboard fixture (batch eight) observed a real behavioral difference:
Ctrl+Z after the first early Backspace leaves Flutter at `ab`, while
Incular restores `abc`. Redo ends at `ab` on both. The probe below
isolates timing from shortcut mapping and reads full text, caret and
history availability, not just length.

| Contract | Flutter: after edit → undo → redo | Incular: after edit → undo → redo |
| --- | --- | --- |
| Same early deletion as batch eight; do not count again | ab → ab → ab | ab → abc → ab |
| Control: wait 600 ms after initial valid selection, then one deletion | ab → abc → ab | ab → abc → ab |
| C40: same seeded baseline; two rapid deletions within one 500 ms window | a → abc → a | a → ab → a |

In the early Flutter case there is no undo or redo at any sampled point.
In the early Incular case there is an undo before the action and a redo
afterwards. After rapid-pair undo, Flutter has caret 3 and only redo;
Incular has caret 2 and still has both undo and redo. The seeded
single-deletion control also matches on caret and history availability.

Flutter filters invalid selections and ignores selection-only updates when
deciding whether to record: `widgets/editable_text.dart:5918–5959`.
`widgets/undo_history.dart:102–105,184–205,231–241,502–515` starts a
500 ms timer and replaces its pending value until it fires. The initial
valid selection and early edit can share a window, leaving only the
post-edit value as the first stack entry. Two rapid later edits also
coalesce. Incular's current policy is to hold the initial controller
value and record each committed text change immediately:
`crates/incular-runtime/src/undo.rs:176–195,249–275,334–370`.
The runtime's focus path supplies this controller
(`frame.rs:1813–1819,1891–1907`) and the keyboard path calls it
(`frame/text_actions.rs:146–155`). The direct Rust contrast uses
committed ASCII text assignments as equivalent post-edit values; it
does not exercise native two-Backspace dispatch. The real keyboard
fixture separately verifies the early-deletion case.

## Reproduction

The original strict keyboard check is
`./audits/flutter-3.50.0-parity/run-focused-frames.ps1 -Scenario @('text__editable_undo_redo')`.
For the timing contrasts, Flutter `test --no-pub test/undo_seed_slice_test.dart`
passed all three; `cargo test --locked -p incular-runtime --test audit_undo_seed_tmp -- --nocapture`
matched the seeded single control and failed the two relevant parity
assertions. Temporary Rust test source was removed.

```dart
import 'package:flutter/widgets.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  void scenario(String name, int delay, int deletes, List<String> expected) {
    testWidgets(name, (t) async {
      final text=TextEditingController(text:'abc');
      final focus=FocusNode();
      final history=UndoHistoryController();
      await t.pumpWidget(Directionality(textDirection:TextDirection.ltr,
        child:DefaultTextEditingShortcuts(child:EditableText(
          controller:text, focusNode:focus, undoController:history,
          style:const TextStyle(), cursorColor:const Color(0xff000000),
          backgroundCursorColor:const Color(0xff000000)))));
      try {
        focus.requestFocus();
        await t.pump();
        text.selection=const TextSelection.collapsed(offset:3);
        await t.pump(Duration(milliseconds:delay));
        for(var i=0;i<deletes;i++){
          await t.sendKeyEvent(LogicalKeyboardKey.backspace);
          await t.pump(const Duration(milliseconds:16));
        }
        await t.pump(const Duration(milliseconds:600));
        String snapshot()=>'${text.text} caret=${text.selection.extentOffset} ${history.value.canUndo}/${history.value.canRedo}';
        final states=[snapshot()];
        final texts=[text.text];
        history.undo();
        await t.pump();
        states.add(snapshot()); texts.add(text.text);
        history.redo();
        await t.pump();
        states.add(snapshot()); texts.add(text.text);
        print(states);
        expect(texts,expected);
      } finally {
        await t.pumpWidget(const SizedBox.shrink());
        history.dispose(); focus.dispose(); text.dispose();
      }
    });
  }
  scenario('unseeded single edit',0,1,['ab','ab','ab']);
  scenario('seeded single edit',600,1,['ab','abc','ab']);
  scenario('seeded rapid pair',600,2,['a','abc','a']);
}
```

```rust
use incular_runtime::UndoHistoryController;
use incular_text::{TextEditingController,TextSelection};
fn run(name:&str, deletes:usize, expected:[&str;3]) {
    let c=TextEditingController::with_text("abc");
    let h=UndoHistoryController::new(c.clone());
    c.set_selection(TextSelection::new(3,3));
    for remaining in (3-deletes..3).rev() {
        c.set_text(&"abc"[..remaining]);
        c.set_selection(TextSelection::new(remaining,remaining));
    }
    let snapshot=||format!("{} caret={} {}/{}",c.text(),c.selection().extent,h.can_undo(),h.can_redo());
    let mut states=vec![snapshot()];
    let mut texts=vec![c.text()];
    let _ = h.undo();
    states.push(snapshot()); texts.push(c.text());
    let _ = h.redo();
    states.push(snapshot()); texts.push(c.text());
    println!("{name}: {states:?}");
    assert_eq!(texts,expected);
}
#[test] fn unseeded_single_edit(){run("unseeded single edit",1,["ab","ab","ab"]);}
#[test] fn seeded_single_edit(){run("seeded single edit",1,["ab","abc","ab"]);}
#[test] fn seeded_rapid_pair(){run("seeded rapid pair",2,["a","abc","a"]);}
```

## Retained 500ms history

Mounted EditableText now coalesces accepted values within a fixed 500ms
window, including its first valid selection. An early deletion can
therefore become the first recorded state rather than an undoable edit.
A seeded pair of quick deletions undoes to the previous committed value
and caret in one step. Keyboard undo cancels an uncommitted group
without creating a redo entry. The retained history still stores
compact deltas; the standalone immediate controller remains available.

A fixed wake deadline, not a continuous redraw request, commits the
group even when the editor is unfocused. Desktop windows feed that
deadline into the existing event-loop wait; the platform-undo controls
check availability only after accepted values with valid selection
commit. Both pinned authored and native timing tests pass. Fresh
strict traces match all eight original undo/redo frames and all nine
physical-End frames.

## Windows redo shortcut control

The older native Ctrl+Y test attempted undo before the initial
500 ms history group had been recorded. It failed at Ctrl+Z and
never checked Ctrl+Y. Both the mounted Rust test and the pinned
authored Flutter variant now first commit the baseline, then commit
the next edit, and send the three actual keyboard chords. Ctrl+Z
restores `abc`, Ctrl+Y does nothing, and Ctrl+Shift+Z restores
`abcd`. All 17 focused native text-shortcut checks passed; the
authored Windows policy probe passed on the pinned Linux runner.
No production shortcut changed or upstream case was added.
