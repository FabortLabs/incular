import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'recorder.dart';

void main() {
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
}
