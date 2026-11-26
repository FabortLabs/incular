import 'dart:ui' as ui;

import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('EditableText clips only overflow and honors each clip mode', (
    tester,
  ) async {
    final controller = TextEditingController();
    final focus = FocusNode();
    addTearDown(controller.dispose);
    addTearDown(focus.dispose);

    for (final clip in Clip.values) {
      for (final overflow in [false, true]) {
        controller.text = overflow ? 'A' * 12 : 'A';
        await tester.pumpWidget(
          Directionality(
            textDirection: TextDirection.ltr,
            child: Center(
              child: SizedBox(
                width: 60,
                height: 30,
                child: EditableText(
                  controller: controller,
                  focusNode: focus,
                  style: const TextStyle(fontSize: 14),
                  cursorColor: const Color(0xff000000),
                  backgroundCursorColor: const Color(0xff000000),
                  clipBehavior: clip,
                ),
              ),
            ),
          ),
        );
        final editable = tester.allRenderObjects
            .whereType<RenderEditable>()
            .single;
        final shouldClip = clip != Clip.none && overflow;
        expect(editable.maxScrollExtent > 0, overflow);
        final expected = shouldClip ? Offset.zero & editable.size : null;
        var didVisit = false;
        editable.visitChildren((child) {
          didVisit = true;
          expect(editable.describeApproximatePaintClip(child), expected);
        });
        expect(didVisit, isTrue);
        void paint(ui.Canvas canvas) {
          editable.paint(TestRecordingPaintingContext(canvas), Offset.zero);
        }

        expect(paint, paintsExactlyCountTimes(#clipRect, shouldClip ? 1 : 0));
        expect(
          paint,
          paintsExactlyCountTimes(
            #saveLayer,
            shouldClip && clip == Clip.antiAliasWithSaveLayer ? 1 : 0,
          ),
        );
      }
    }
  });
}
