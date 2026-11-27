import 'package:flutter/gestures.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Span cursor and transitions respect the enclosing region', (
    WidgetTester tester,
  ) async {
    final events = <String>[];
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Center(
          child: MouseRegion(
            cursor: SystemMouseCursors.click,
            onEnter: (_) => events.add('region enter'),
            onExit: (_) => events.add('region exit'),
            child: Text.rich(
              TextSpan(
                text: 'xxxxx',
                children: [
                  TextSpan(
                    text: 'yyyyy',
                    mouseCursor: SystemMouseCursors.forbidden,
                    onEnter: (_) => events.add('span enter'),
                    onExit: (_) => events.add('span exit'),
                  ),
                  const TextSpan(text: 'xxxxx'),
                ],
              ),
              style: const TextStyle(fontSize: 10),
            ),
          ),
        ),
      ),
    );
    final center = tester.getCenter(find.byType(RichText));
    final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
    await mouse.addPointer();
    await mouse.moveTo(center - const Offset(40, 0));
    expect(
      RendererBinding.instance.mouseTracker.debugDeviceActiveCursor(1),
      SystemMouseCursors.click,
    );
    await mouse.moveTo(center);
    expect(
      RendererBinding.instance.mouseTracker.debugDeviceActiveCursor(1),
      SystemMouseCursors.forbidden,
    );
    await mouse.moveTo(center + const Offset(0, 25));
    expect(events, ['region enter', 'span enter', 'span exit', 'region exit']);
    await mouse.removePointer();
  });

  testWidgets(
    'The ellipsis does not belong to the truncated interactive span',
    (WidgetTester tester) async {
      await tester.pumpWidget(
        const Directionality(
          textDirection: TextDirection.ltr,
          child: Center(
            child: SizedBox(
              width: 80,
              child: Text.rich(
                TextSpan(
                  text: 'xxxxx',
                  children: [
                    TextSpan(
                      text: 'yyyyyyyyyy',
                      mouseCursor: SystemMouseCursors.forbidden,
                    ),
                  ],
                ),
                style: TextStyle(fontSize: 10),
                overflow: TextOverflow.ellipsis,
                maxLines: 1,
              ),
            ),
          ),
        ),
      );
      final origin = tester.getTopLeft(find.byType(RichText));
      final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
      await mouse.addPointer();
      await mouse.moveTo(origin + const Offset(55, 5));
      expect(
        RendererBinding.instance.mouseTracker.debugDeviceActiveCursor(1),
        SystemMouseCursors.forbidden,
      );
      await mouse.moveTo(origin + const Offset(75, 5));
      expect(
        RendererBinding.instance.mouseTracker.debugDeviceActiveCursor(1),
        SystemMouseCursors.basic,
      );
      await mouse.removePointer();
    },
  );
  testWidgets('Removing a hovered span still dispatches its exit', (
    WidgetTester tester,
  ) async {
    var exits = 0;
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Center(
          child: Text.rich(
            TextSpan(text: 'yyyyy', onExit: (_) => exits++),
            style: const TextStyle(fontSize: 10),
          ),
        ),
      ),
    );
    final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
    await mouse.addPointer();
    await mouse.moveTo(tester.getCenter(find.byType(RichText)));
    expect(exits, 0);
    await tester.pumpWidget(const SizedBox.shrink());
    expect(exits, 1);
    await mouse.removePointer();
  });
}
