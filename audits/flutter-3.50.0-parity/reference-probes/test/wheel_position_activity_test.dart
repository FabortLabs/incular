import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('wheel listeners see a moving position inside the activity edge', (
    tester,
  ) async {
    final controller = ScrollController();
    addTearDown(controller.dispose);
    late ScrollPosition position;
    final trace = <(String, double, bool)>[];
    void record(String event) {
      trace.add((event, position.pixels, position.isScrollingNotifier.value));
    }

    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: NotificationListener<ScrollNotification>(
          onNotification: (event) {
            switch (event) {
              case ScrollStartNotification():
                record('start');
              case ScrollUpdateNotification():
                record('update');
              case ScrollEndNotification():
                record('end');
              default:
                break;
            }
            return false;
          },
          child: ListView(
            controller: controller,
            children: const [SizedBox(height: 2000)],
          ),
        ),
      ),
    );
    position = controller.position;
    position.isScrollingNotifier.addListener(() => record('scrolling'));
    position.addListener(() => record('position'));

    await tester.sendEventToBinding(
      const PointerScrollEvent(position: Offset(30, 30), scrollDelta: Offset(0, 25)),
    );
    expect(trace, [
      ('scrolling', 0, true),
      ('position', 25, true),
      ('start', 25, true),
      ('update', 25, true),
      ('end', 25, true),
      ('scrolling', 25, false),
    ]);
    await tester.pump();
    expect(trace, hasLength(6));
  });
}
