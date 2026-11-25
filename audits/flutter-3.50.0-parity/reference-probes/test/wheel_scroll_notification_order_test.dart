import 'package:flutter/gestures.dart' show PointerDeviceKind;
import 'package:flutter/rendering.dart' show ScrollDirection;
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final physics in [
    const ClampingScrollPhysics(),
    const BouncingScrollPhysics(),
  ]) {
    testWidgets(
      'wheel directions surround a single committed scroll bracket: ${physics.runtimeType}',
      (tester) async {
        final controller = ScrollController();
        addTearDown(controller.dispose);
        final trace = <(String, double)>[];
        await tester.pumpWidget(
          Directionality(
            textDirection: TextDirection.ltr,
            child: NotificationListener<ScrollNotification>(
              onNotification: (notification) {
                final name = switch (notification) {
                  UserScrollNotification(:final direction) => direction.name,
                  ScrollStartNotification() => 'start',
                  ScrollUpdateNotification() => 'update',
                  ScrollEndNotification() => 'end',
                  OverscrollNotification() => 'overscroll',
                  _ => '?',
                };
                trace.add((name, notification.metrics.pixels));
                return false;
              },
              child: ListView.builder(
                controller: controller,
                physics: physics,
                itemExtent: 50,
                itemCount: 40,
                itemBuilder: (_, _) => const SizedBox.shrink(),
              ),
            ),
          ),
        );
        final mouse = TestPointer(1, PointerDeviceKind.mouse);
        await tester.sendEventToBinding(
          mouse.addPointer(location: const Offset(400, 300)),
        );
        Future<void> wheel(double dy) async {
          await tester.sendEventToBinding(mouse.scroll(Offset(0, dy)));
          await tester.pump();
        }

        await wheel(-40);
        expect(
          trace,
          isEmpty,
          reason: 'a pinned edge does not start a bracket',
        );
        await wheel(60);
        expect(trace, [
          (ScrollDirection.reverse.name, 0),
          ('start', 60),
          ('update', 60),
          ('end', 60),
          (ScrollDirection.idle.name, 60),
        ]);
        trace.clear();
        await wheel(-90);
        expect(trace, [
          (ScrollDirection.forward.name, 60),
          ('start', 0),
          ('update', 0),
          ('end', 0),
          (ScrollDirection.idle.name, 0),
        ]);
      },
    );
  }
}
