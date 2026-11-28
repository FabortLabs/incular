import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('a custom editor group recognizes only its own companions', (
    tester,
  ) async {
    final controller = TextEditingController(text: 'ab');
    final node = FocusNode();
    final events = <String>[];
    final matching = UniqueKey();
    final foreign = UniqueKey();
    addTearDown(controller.dispose);
    addTearDown(node.dispose);
    await tester.pumpWidget(
      MediaQuery(
        data: const MediaQueryData(),
        child: Directionality(
          textDirection: TextDirection.ltr,
          child: TapRegionSurface(
            child: Row(
              children: [
                TapRegion(
                  groupId: 'Group A',
                  behavior: HitTestBehavior.opaque,
                  child: SizedBox(key: matching, width: 100, height: 40),
                ),
                SizedBox(
                  width: 100,
                  child: EditableText(
                    controller: controller,
                    focusNode: node,
                    groupId: 'Group A',
                    style: const TextStyle(fontSize: 14),
                    cursorColor: const Color(0xFF2196F3),
                    backgroundCursorColor: const Color(0xFF9E9E9E),
                    onTapOutside: (_) => events.add('down'),
                    onTapUpOutside: (_) => events.add('up'),
                  ),
                ),
                TextFieldTapRegion(
                  groupId: 'Group B',
                  child: SizedBox(
                    key: foreign,
                    width: 100,
                    height: 40,
                    child: const ColoredBox(color: Color(0xFFFFFFFF)),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
    node.requestFocus();
    await tester.pumpAndSettle();
    expect(node.hasPrimaryFocus, isTrue);
    final a = tester.getCenter(find.byKey(matching));
    final b = tester.getCenter(find.byKey(foreign));

    final inside = await tester.startGesture(a, kind: PointerDeviceKind.mouse);
    await inside.moveTo(b);
    await inside.up();
    expect(events, isEmpty);

    final outside = await tester.startGesture(b, kind: PointerDeviceKind.mouse);
    expect(events, ['down']);
    await outside.moveTo(a);
    await outside.up();
    expect(events, ['down', 'up']);
    expect(node.hasPrimaryFocus, isTrue);
    await tester.pumpWidget(const SizedBox());
  });
}
