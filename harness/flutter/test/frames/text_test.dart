import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'recorder.dart';

void main() {
  registerKeyScenarios();
  registerFinalScenarios();
  frameScenario('text__rich_span_style_metrics', (h) async {
    await h.mount(
      (_) => Align(
        alignment: Alignment.topLeft,
        child: SizedBox(
          width: 220,
          child: h.probe(
            'paragraph',
            RichText(
              textDirection: TextDirection.ltr,
              text: const TextSpan(
                text: 'Small ',
                style: TextStyle(
                  fontFamily: 'Inter',
                  fontSize: 14,
                  color: Colors.black,
                ),
                children: <InlineSpan>[
                  TextSpan(text: 'Large', style: TextStyle(fontSize: 30)),
                ],
              ),
            ),
          ),
        ),
      ),
    );
    await h.frames(1);
  });

  frameScenario('text__widget_span_layout', (h) async {
    await h.mount(
      (_) => Align(
        alignment: Alignment.topLeft,
        child: SizedBox(
          width: 220,
          child: h.probe(
            'paragraph',
            RichText(
              textDirection: TextDirection.ltr,
              text: const TextSpan(
                text: 'before ',
                style: TextStyle(
                  fontFamily: 'Inter',
                  fontSize: 14,
                  color: Colors.black,
                ),
                children: <InlineSpan>[
                  WidgetSpan(
                    alignment: PlaceholderAlignment.middle,
                    child: SizedBox(
                      width: 40,
                      height: 40,
                      child: ColoredBox(color: Colors.blue),
                    ),
                  ),
                  TextSpan(text: ' after'),
                ],
              ),
            ),
          ),
        ),
      ),
    );
    await h.frames(1);
  });

  frameScenario('text__editing_selection_offsets', (h) async {
    const text = '😀x';
    final controller = TextEditingController.fromValue(
      TextEditingValue(
        text: text,
        selection: TextSelection.collapsed(offset: text.length),
      ),
    );
    final focusNode = FocusNode();
    h.value('text_length', () => controller.text.length.toDouble());
    h.value(
      'selection_extent',
      () => controller.selection.extentOffset.toDouble(),
    );
    await h.mount(
      (_) => Align(
        alignment: Alignment.topLeft,
        child: SizedBox(
          width: 200,
          height: 48,
          child: h.probe(
            'field',
            DefaultTextEditingShortcuts(
              child: EditableText(
                controller: controller,
                focusNode: focusNode,
                style: const TextStyle(
                  fontFamily: 'Inter',
                  fontSize: 14,
                  color: Colors.black,
                ),
                cursorColor: Colors.black,
                backgroundCursorColor: Colors.black,
              ),
            ),
          ),
        ),
      ),
    );
    await h.frames(1);
    focusNode.dispose();
    controller.dispose();
  });

  frameScenario('text__default_text_style_max_lines', (h) async {
    await h.mount(
      (_) => Align(
        alignment: Alignment.topLeft,
        child: SizedBox(
          width: 220,
          child: DefaultTextStyle(
            style: const TextStyle(
              fontFamily: 'Inter',
              fontSize: 14,
              color: Colors.black,
            ),
            maxLines: 1,
            child: h.probe('paragraph', const Text('first\nsecond')),
          ),
        ),
      ),
    );
    await h.frames(1);
  });

  frameScenario('text__editable_initial_selection_focus', (h) async {
    final controller = TextEditingController(text: 'abc');
    final focusNode = FocusNode();
    h.value('selection_valid', () => controller.selection.isValid ? 1 : 0);
    h.value(
      'selection_collapsed',
      () => controller.selection.isCollapsed ? 1 : 0,
    );
    h.value(
      'selection_extent',
      () => controller.selection.extentOffset.toDouble(),
    );
    await h.mount(
      (_) => Align(
        alignment: Alignment.topLeft,
        child: SizedBox(
          width: 200,
          height: 48,
          child: h.probe(
            'field',
            EditableText(
              controller: controller,
              focusNode: focusNode,
              style: const TextStyle(
                fontFamily: 'Inter',
                fontSize: 14,
                color: Colors.black,
              ),
              cursorColor: Colors.black,
              backgroundCursorColor: Colors.black,
            ),
          ),
        ),
      ),
    );
    await h.frames(1);
    await h.down(h.center('field'));
    await h.up();
    await h.frames(1);
    focusNode.dispose();
    controller.dispose();
  });

  frameScenario('text__editable_programmatic_controller_update', (h) async {
    final controller = TextEditingController();
    final focusNode = FocusNode();
    var changedCalls = 0;
    h.value('text_length', () => controller.text.length.toDouble());
    h.value('changed_calls', () => changedCalls.toDouble());
    await h.mount(
      (_) => Align(
        alignment: Alignment.topLeft,
        child: SizedBox(
          width: 200,
          height: 48,
          child: h.probe(
            'field',
            EditableText(
              controller: controller,
              focusNode: focusNode,
              style: const TextStyle(
                fontFamily: 'Inter',
                fontSize: 14,
                color: Colors.black,
              ),
              cursorColor: Colors.black,
              backgroundCursorColor: Colors.black,
              inputFormatters: <TextInputFormatter>[
                FilteringTextInputFormatter.digitsOnly,
              ],
              onChanged: (_) => changedCalls += 1,
            ),
          ),
        ),
      ),
    );
    await h.frames(1);
    controller.text = 'a1b2';
    await h.frames(1);
    focusNode.dispose();
    controller.dispose();
  });

  frameScenario('text__editable_long_press_word_selection', (h) async {
    final controller = TextEditingController.fromValue(
      const TextEditingValue(
        text: 'hello world',
        selection: TextSelection.collapsed(offset: 11),
      ),
    );
    final focusNode = FocusNode();
    h.value('selection_start', () => controller.selection.start.toDouble());
    h.value('selection_end', () => controller.selection.end.toDouble());
    h.value(
      'selection_collapsed',
      () => controller.selection.isCollapsed ? 1 : 0,
    );
    await h.mount(
      (_) => Align(
        alignment: Alignment.topLeft,
        child: SizedBox(
          width: 200,
          height: 48,
          child: h.probe(
            'field',
            EditableText(
              controller: controller,
              focusNode: focusNode,
              style: const TextStyle(
                fontFamily: 'Inter',
                fontSize: 14,
                color: Colors.black,
              ),
              cursorColor: Colors.black,
              backgroundCursorColor: Colors.black,
            ),
          ),
        ),
      ),
    );
    await h.frames(1);
    await h.down(const Offset(16, 24));
    await h.pump(const Duration(milliseconds: 600));
    await h.up();
    await h.frames(1);
    focusNode.dispose();
    controller.dispose();
  });
  frameScenario('text__text_max_lines_ellipsis', (h) async {
    await h.mount(
      (_) => Align(
        alignment: Alignment.topLeft,
        child: SizedBox(
          width: 100,
          child: h.probe(
            'paragraph',
            const Text(
              'alpha beta gamma delta epsilon',
              style: TextStyle(
                fontFamily: 'Inter',
                fontSize: 14,
                color: Colors.black,
              ),
              maxLines: 2,
              overflow: TextOverflow.ellipsis,
            ),
          ),
        ),
      ),
    );
    await h.frames(1);
  });
}

const TextStyle _fieldStyle = TextStyle(
  fontFamily: 'Inter',
  fontSize: 14,
  color: Colors.black,
);

Future<void> _chord(
  FrameHarness h,
  List<LogicalKeyboardKey> modifiers,
  LogicalKeyboardKey key,
) async {
  for (final modifier in modifiers) {
    await h.keyDown(modifier);
  }
  await h.key(key);
  for (final modifier in modifiers.reversed) {
    await h.keyUp(modifier);
  }
}

Widget _plainField(TextEditingController controller, FocusNode focusNode) =>
    DefaultTextEditingShortcuts(
      child: EditableText(
      controller: controller,
      focusNode: focusNode,
      style: _fieldStyle,
      cursorColor: Colors.black,
      backgroundCursorColor: Colors.black,
      ),
    );

void registerKeyScenarios() {
  frameScenario('text__editable_word_arrow_selection', (h) async {
    final controller = TextEditingController(text: 'hello big world');
    final focusNode = FocusNode();
    h.value('selection_base', () => controller.selection.baseOffset.toDouble());
    h.value('selection_extent', () => controller.selection.extentOffset.toDouble());
    await h.mount(
      (_) => Align(
        alignment: Alignment.topLeft,
        child: SizedBox(
          width: 200,
          height: 48,
          child: h.probe('field', _plainField(controller, focusNode)),
        ),
      ),
    );
    await h.frames(1);
    await h.down(h.center('field'));
    await h.up();
    await h.frames(1);
    controller.selection = const TextSelection.collapsed(offset: 0);
    await h.frames(1);
    await _chord(h, [LogicalKeyboardKey.controlLeft], LogicalKeyboardKey.arrowRight);
    await h.frames(1);
    await _chord(h, [LogicalKeyboardKey.controlLeft], LogicalKeyboardKey.arrowRight);
    await h.frames(1);
    await _chord(h, [LogicalKeyboardKey.shiftLeft], LogicalKeyboardKey.arrowRight);
    await h.frames(1);
    focusNode.dispose();
    controller.dispose();
  });

  frameScenario('text__editable_home_end_shift_select', (h) async {
    final controller = TextEditingController(text: 'hello world');
    final focusNode = FocusNode();
    h.value('selection_base', () => controller.selection.baseOffset.toDouble());
    h.value('selection_extent', () => controller.selection.extentOffset.toDouble());
    await h.mount(
      (_) => Align(
        alignment: Alignment.topLeft,
        child: SizedBox(
          width: 200,
          height: 48,
          child: h.probe('field', _plainField(controller, focusNode)),
        ),
      ),
    );
    await h.frames(1);
    await h.down(h.center('field'));
    await h.up();
    await h.frames(1);
    controller.selection = const TextSelection.collapsed(offset: 5);
    await h.frames(1);
    await _chord(h, const [], LogicalKeyboardKey.end);
    await h.frames(1);
    await _chord(h, [LogicalKeyboardKey.shiftLeft], LogicalKeyboardKey.home);
    await h.frames(1);
    focusNode.dispose();
    controller.dispose();
  });

  frameScenario('text__editable_backspace_delete', (h) async {
    final controller = TextEditingController(text: 'abcdef');
    final focusNode = FocusNode();
    h.value('text_length', () => controller.text.length.toDouble());
    h.value('selection_extent', () => controller.selection.extentOffset.toDouble());
    await h.mount(
      (_) => Align(
        alignment: Alignment.topLeft,
        child: SizedBox(
          width: 200,
          height: 48,
          child: h.probe('field', _plainField(controller, focusNode)),
        ),
      ),
    );
    await h.frames(1);
    await h.down(h.center('field'));
    await h.up();
    await h.frames(1);
    controller.selection = const TextSelection.collapsed(offset: 3);
    await h.frames(1);
    await _chord(h, const [], LogicalKeyboardKey.backspace);
    await h.frames(1);
    await _chord(h, const [], LogicalKeyboardKey.delete);
    await h.frames(1);
    focusNode.dispose();
    controller.dispose();
  });

  frameScenario('text__editable_undo_redo', (h) async {
    final controller = TextEditingController(text: 'abc');
    final focusNode = FocusNode();
    h.value('text_length', () => controller.text.length.toDouble());
    await h.mount(
      (_) => Align(
        alignment: Alignment.topLeft,
        child: SizedBox(
          width: 200,
          height: 48,
          child: h.probe('field', _plainField(controller, focusNode)),
        ),
      ),
    );
    await h.frames(1);
    await h.down(h.center('field'));
    await h.up();
    await h.frames(1);
    controller.selection = const TextSelection.collapsed(offset: 3);
    await h.frames(1);
    await _chord(h, const [], LogicalKeyboardKey.backspace);
    await h.frames(1);
    await h.pump(const Duration(milliseconds: 600));
    await _chord(h, [LogicalKeyboardKey.controlLeft], LogicalKeyboardKey.keyZ);
    await h.frames(1);
    await _chord(
      h,
      [LogicalKeyboardKey.controlLeft, LogicalKeyboardKey.shiftLeft],
      LogicalKeyboardKey.keyZ,
    );
    await h.frames(1);
    focusNode.dispose();
    controller.dispose();
  });

  frameScenario('text__editable_enter_callback_order', (h) async {
    final controller = TextEditingController();
    final focusNode = FocusNode();
    var changed = 0;
    var submitted = 0;
    var editingComplete = 0;
    h.tester.testTextInput.register();
    h.value('changed_calls', () => changed.toDouble());
    h.value('submitted_calls', () => submitted.toDouble());
    h.value('editing_complete_calls', () => editingComplete.toDouble());
    h.value('text_length', () => controller.text.length.toDouble());
    await h.mount(
      (_) => Align(
        alignment: Alignment.topLeft,
        child: SizedBox(
          width: 200,
          height: 48,
          child: h.probe(
            'field',
            EditableText(
              controller: controller,
              focusNode: focusNode,
              style: _fieldStyle,
              cursorColor: Colors.black,
              backgroundCursorColor: Colors.black,
              onChanged: (_) => changed += 1,
              onSubmitted: (_) => submitted += 1,
              onEditingComplete: () => editingComplete += 1,
            ),
          ),
        ),
      ),
    );
    await h.frames(1);
    await h.down(h.center('field'));
    await h.up();
    await h.frames(1);
    h.tester.testTextInput.enterText('ab');
    await h.frames(1);
    await h.key(LogicalKeyboardKey.enter);
    await h.frames(1);
    focusNode.dispose();
    controller.dispose();
  });

  frameScenario('text__editable_formatter_length_then_digits', (h) async {
    final controller = TextEditingController();
    final focusNode = FocusNode();
    h.tester.testTextInput.register();
    h.value('text_length', () => controller.text.length.toDouble());
    await h.mount(
      (_) => Align(
        alignment: Alignment.topLeft,
        child: SizedBox(
          width: 200,
          height: 48,
          child: h.probe(
            'field',
            EditableText(
              controller: controller,
              focusNode: focusNode,
              style: _fieldStyle,
              cursorColor: Colors.black,
              backgroundCursorColor: Colors.black,
              inputFormatters: <TextInputFormatter>[
                LengthLimitingTextInputFormatter(3),
                FilteringTextInputFormatter.digitsOnly,
              ],
            ),
          ),
        ),
      ),
    );
    await h.frames(1);
    await h.down(h.center('field'));
    await h.up();
    await h.frames(1);
    h.tester.testTextInput.enterText('a1b2c3d4');
    await h.frames(1);
    focusNode.dispose();
    controller.dispose();
  });

  frameScenario('text__editable_formatter_digits_then_length', (h) async {
    final controller = TextEditingController();
    final focusNode = FocusNode();
    h.tester.testTextInput.register();
    h.value('text_length', () => controller.text.length.toDouble());
    await h.mount(
      (_) => Align(
        alignment: Alignment.topLeft,
        child: SizedBox(
          width: 200,
          height: 48,
          child: h.probe(
            'field',
            EditableText(
              controller: controller,
              focusNode: focusNode,
              style: _fieldStyle,
              cursorColor: Colors.black,
              backgroundCursorColor: Colors.black,
              inputFormatters: <TextInputFormatter>[
                FilteringTextInputFormatter.digitsOnly,
                LengthLimitingTextInputFormatter(3),
              ],
            ),
          ),
        ),
      ),
    );
    await h.frames(1);
    await h.down(h.center('field'));
    await h.up();
    await h.frames(1);
    h.tester.testTextInput.enterText('a1b2c3d4');
    await h.frames(1);
    focusNode.dispose();
    controller.dispose();
  });

  frameScenario('text__editable_read_only_typing', (h) async {
    final controller = TextEditingController(text: 'abc');
    final focusNode = FocusNode();
    h.tester.testTextInput.register();
    h.value('text_length', () => controller.text.length.toDouble());
    await h.mount(
      (_) => Align(
        alignment: Alignment.topLeft,
        child: SizedBox(
          width: 200,
          height: 48,
          child: h.probe(
            'field',
            EditableText(
              controller: controller,
              focusNode: focusNode,
              style: _fieldStyle,
              cursorColor: Colors.black,
              backgroundCursorColor: Colors.black,
              readOnly: true,
            ),
          ),
        ),
      ),
    );
    await h.frames(1);
    await h.down(h.center('field'));
    await h.up();
    await h.frames(1);
    h.tester.testTextInput.enterText('xyz');
    await h.frames(1);
    focusNode.dispose();
    controller.dispose();
  });

  frameScenario('text__editable_max_lines_sizing', (h) async {
    final controller = TextEditingController(text: 'a\nb\nc\nd\ne');
    final focusNode = FocusNode();
    await h.mount(
      (_) => Align(
        alignment: Alignment.topLeft,
        child: SizedBox(
          width: 200,
          child: h.probe(
            'field',
            EditableText(
              controller: controller,
              focusNode: focusNode,
              style: _fieldStyle,
              cursorColor: Colors.black,
              backgroundCursorColor: Colors.black,
              maxLines: 3,
            ),
          ),
        ),
      ),
    );
    await h.frames(1);
    focusNode.dispose();
    controller.dispose();
  });

  frameScenario('text__editable_min_lines_sizing', (h) async {
    final controller = TextEditingController(text: 'a');
    final focusNode = FocusNode();
    await h.mount(
      (_) => Align(
        alignment: Alignment.topLeft,
        child: SizedBox(
          width: 200,
          child: h.probe(
            'field',
            EditableText(
              controller: controller,
              focusNode: focusNode,
              style: _fieldStyle,
              cursorColor: Colors.black,
              backgroundCursorColor: Colors.black,
              minLines: 2,
              maxLines: 4,
            ),
          ),
        ),
      ),
    );
    await h.frames(1);
    focusNode.dispose();
    controller.dispose();
  });

  frameScenario('text__editable_expands_sizing', (h) async {
    final controller = TextEditingController(text: 'a\nb');
    final focusNode = FocusNode();
    await h.mount(
      (_) => Align(
        alignment: Alignment.topLeft,
        child: SizedBox(
          width: 200,
          height: 120,
          child: h.probe(
            'field',
            EditableText(
              controller: controller,
              focusNode: focusNode,
              style: _fieldStyle,
              cursorColor: Colors.black,
              backgroundCursorColor: Colors.black,
              maxLines: null,
              expands: true,
            ),
          ),
        ),
      ),
    );
    await h.frames(1);
    focusNode.dispose();
    controller.dispose();
  });

  frameScenario('text__text_soft_wrap_false', (h) async {
    await h.mount(
      (_) => Align(
        alignment: Alignment.topLeft,
        child: SizedBox(
          width: 60,
          child: h.probe(
            'paragraph',
            const Text('alpha beta gamma', style: _fieldStyle, softWrap: false),
          ),
        ),
      ),
    );
    await h.frames(1);
  });

  frameScenario('text__text_scaler_linear', (h) async {
    await h.mount(
      (_) => Align(
        alignment: Alignment.topLeft,
        child: h.probe(
          'paragraph',
          const Text(
            'alpha',
            style: _fieldStyle,
            textScaler: TextScaler.linear(1.5),
          ),
        ),
      ),
    );
    await h.frames(1);
  });

  frameScenario('text__text_height_behavior_first_ascent', (h) async {
    await h.mount(
      (_) => Align(
        alignment: Alignment.topLeft,
        child: SizedBox(
          width: 200,
          child: h.probe(
            'paragraph',
            const Text(
              'alpha',
              style: TextStyle(
                fontFamily: 'Inter',
                fontSize: 14,
                color: Colors.black,
                height: 2,
              ),
              textHeightBehavior: TextHeightBehavior(
                applyHeightToFirstAscent: false,
              ),
            ),
          ),
        ),
      ),
    );
    await h.frames(1);
  });

  frameScenario('text__selection_area_long_press_drag', (h) async {
    var changes = 0;
    var selectedLength = 0;
    h.value('selection_changes', () => changes.toDouble());
    h.value('selected_length', () => selectedLength.toDouble());
    await h.mount(
      (_) => Localizations(
        locale: const Locale('en', 'US'),
        delegates: const <LocalizationsDelegate<dynamic>>[
          DefaultMaterialLocalizations.delegate,
          DefaultWidgetsLocalizations.delegate,
        ],
        child: Overlay(
          initialEntries: <OverlayEntry>[
            OverlayEntry(
              builder: (_) => Align(
                alignment: Alignment.topLeft,
                child: SizedBox(
                  width: 200,
                  child: SelectionArea(
                    onSelectionChanged: (content) {
                      changes += 1;
                      selectedLength = content?.plainText.length ?? 0;
                    },
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: <Widget>[
                        h.probe(
                          'first',
                          const Text('alpha beta', style: _fieldStyle),
                        ),
                        h.probe(
                          'second',
                          const Text('gamma delta', style: _fieldStyle),
                        ),
                      ],
                    ),
                  ),
                ),
              ),
            ),
          ],
        ),
      ),
    );
    await h.frames(1);
    await h.down(h.center('first'));
    await h.pump(const Duration(milliseconds: 600));
    await h.move(h.center('second'));
    await h.frames(1);
    await h.up();
    await h.frames(1);
  });
}

void registerFinalScenarios() {
  frameScenario('text__editable_undo_no_programmatic_selection', (h) async {
    final controller = TextEditingController(text: 'abc');
    final focusNode = FocusNode();
    h.value('text_length', () => controller.text.length.toDouble());
    await h.mount(
      (_) => Align(
        alignment: Alignment.topLeft,
        child: SizedBox(
          width: 200,
          height: 48,
          child: h.probe('field', _plainField(controller, focusNode)),
        ),
      ),
    );
    await h.frames(1);
    await h.down(h.center('field'));
    await h.up();
    await h.frames(1);
    await _chord(h, const [], LogicalKeyboardKey.end);
    await h.frames(1);
    await _chord(h, const [], LogicalKeyboardKey.backspace);
    await h.frames(1);
    await h.pump(const Duration(milliseconds: 600));
    await _chord(h, [LogicalKeyboardKey.controlLeft], LogicalKeyboardKey.keyZ);
    await h.frames(1);
    await h.pump(const Duration(milliseconds: 600));
    await _chord(
      h,
      [LogicalKeyboardKey.controlLeft, LogicalKeyboardKey.shiftLeft],
      LogicalKeyboardKey.keyZ,
    );
    await h.frames(1);
    focusNode.dispose();
    controller.dispose();
  });

  frameScenario('text__editable_caret_blink_1200ms', (h) async {
    final controller = TextEditingController(text: 'abc');
    final focusNode = FocusNode();
    h.value('caret_visible', () {
      final state = h.tester.state<EditableTextState>(
        find.byType(EditableText),
      );
      return state.renderEditable.showCursor.value ? 1 : 0;
    });
    await h.mount(
      (_) => Align(
        alignment: Alignment.topLeft,
        child: SizedBox(
          width: 200,
          height: 48,
          child: h.probe('field', _plainField(controller, focusNode)),
        ),
      ),
    );
    await h.frames(1);
    await h.down(h.center('field'));
    await h.up();
    await h.frames(75);
    focusNode.dispose();
    controller.dispose();
  });

  frameScenario('text__editable_multiline_vertical_document', (h) async {
    final controller = TextEditingController(text: 'abc\ndefgh\nij');
    final focusNode = FocusNode();
    h.value('selection_base', () => controller.selection.baseOffset.toDouble());
    h.value('selection_extent', () => controller.selection.extentOffset.toDouble());
    await h.mount(
      (_) => Align(
        alignment: Alignment.topLeft,
        child: SizedBox(
          width: 200,
          height: 60,
          child: h.probe(
            'field',
            DefaultTextEditingShortcuts(
              child: EditableText(
                controller: controller,
                focusNode: focusNode,
                style: _fieldStyle,
                cursorColor: Colors.black,
                backgroundCursorColor: Colors.black,
                maxLines: 3,
              ),
            ),
          ),
        ),
      ),
    );
    await h.frames(1);
    await h.down(h.center('field'));
    await h.up();
    await h.frames(1);
    controller.selection = const TextSelection.collapsed(offset: 1);
    await h.frames(1);
    await _chord(h, const [], LogicalKeyboardKey.arrowDown);
    await h.frames(1);
    await _chord(h, const [], LogicalKeyboardKey.arrowDown);
    await h.frames(1);
    await _chord(h, const [], LogicalKeyboardKey.arrowUp);
    await h.frames(1);
    await _chord(h, [LogicalKeyboardKey.controlLeft], LogicalKeyboardKey.end);
    await h.frames(1);
    await _chord(h, [LogicalKeyboardKey.controlLeft], LogicalKeyboardKey.home);
    await h.frames(1);
    focusNode.dispose();
    controller.dispose();
  });
}
