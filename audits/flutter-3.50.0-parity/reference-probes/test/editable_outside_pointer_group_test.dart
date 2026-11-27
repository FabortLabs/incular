import 'dart:ui' as ui;

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'A custom desktop outside callback replaces blur without treating the editor group as outside',
    (WidgetTester tester) async {
      final focus = FocusNode();
      final controller = TextEditingController(text: 'text');
      addTearDown(focus.dispose);
      addTearDown(controller.dispose);
      final editor = GlobalKey();
      final outside = GlobalKey();
      final grouped = GlobalKey();
      final events = <PointerDownEvent>[];
      final releases = <PointerUpEvent>[];

      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: TapRegionSurface(
            child: Row(
              children: [
                ColoredBox(
                  color: const Color(0xff000000),
                  child: SizedBox(key: outside, width: 100, height: 40),
                ),
                SizedBox(
                  width: 100,
                  height: 40,
                  child: EditableText(
                    key: editor,
                    controller: controller,
                    focusNode: focus,
                    autofocus: true,
                    style: const TextStyle(),
                    cursorColor: const Color(0xffffffff),
                    backgroundCursorColor: const Color(0xff808080),
                    onTapOutside: events.add,
                    onTapUpOutside: releases.add,
                  ),
                ),
                TextFieldTapRegion(
                  child: ColoredBox(
                    color: const Color(0xffffffff),
                    child: SizedBox(key: grouped, width: 100, height: 40),
                  ),
                ),
              ],
            ),
          ),
        ),
      );
      await tester.pump();
      expect(focus.hasPrimaryFocus, isTrue);

      await tester.tapAt(tester.getCenter(find.byKey(grouped)), kind: ui.PointerDeviceKind.mouse);
      expect(events, isEmpty);
      expect(focus.hasPrimaryFocus, isTrue);

      final outsidePoint = tester.getCenter(find.byKey(outside));
      await tester.tapAt(outsidePoint, kind: ui.PointerDeviceKind.mouse);
      expect(events.single.position, outsidePoint);
      expect(focus.hasPrimaryFocus, isTrue);
      expect(releases.single.position, outsidePoint);
      releases.clear();

      final insidePoint = tester.getCenter(find.byKey(editor));
      final pointer = await tester.createGesture(kind: ui.PointerDeviceKind.mouse);
      await pointer.down(insidePoint);
      await pointer.moveTo(outsidePoint);
      await pointer.up();
      expect(releases, isEmpty);
      await pointer.down(outsidePoint);
      await pointer.moveTo(insidePoint);
      await pointer.up();
      expect(releases.single.position, insidePoint);
      await pointer.removePointer();
    },
    variant: const TargetPlatformVariant({TargetPlatform.linux}),
  );
}
