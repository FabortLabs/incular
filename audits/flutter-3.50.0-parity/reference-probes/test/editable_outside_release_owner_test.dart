import 'package:flutter/foundation.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'outside release belongs to the editor that held the original press',
    (tester) async {
      final text = TextEditingController(text: 'text');
      final focus = FocusNode();
      addTearDown(text.dispose);
      addTearDown(focus.dispose);
      const key = Key('outside');
      final releases = <Offset>[];
      await tester.pumpWidget(
        WidgetsApp(
          color: const Color(0xff000000),
          builder: (_, _) => TapRegionSurface(
            child: Column(
              children: [
                const ColoredBox(
                  key: key,
                  color: Color(0xff000000),
                  child: SizedBox(width: 100, height: 40),
                ),
                EditableText(
                  controller: text,
                  focusNode: focus,
                  autofocus: true,
                  style: const TextStyle(),
                  cursorColor: const Color(0xff000000),
                  backgroundCursorColor: const Color(0xff000000),
                  onTapUpOutside: (event) => releases.add(event.position),
                ),
              ],
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(focus.hasPrimaryFocus, isTrue);
      final at = tester.getCenter(find.byKey(key));
      final press = await tester.startGesture(
        at,
        kind: PointerDeviceKind.mouse,
      );
      await tester.pump();
      expect(focus.hasPrimaryFocus, isFalse);
      expect(releases, isEmpty);
      await press.up();
      await tester.pump();
      expect(releases, [at]);

      final second = await tester.startGesture(
        at,
        kind: PointerDeviceKind.mouse,
      );
      focus.requestFocus();
      await tester.pump();
      expect(focus.hasPrimaryFocus, isTrue);
      await second.up();
      await tester.pump();
      expect(releases, [at]);
    },
    variant: TargetPlatformVariant.only(TargetPlatform.linux),
  );
}
