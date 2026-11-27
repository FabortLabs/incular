import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'opting out of an outer transform keeps nested hits and real clips',
    (tester) async {
      tester.view.physicalSize = const Size(40, 20);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.reset);
      final reported = <Offset>[];
      final scene = Transform.translate(
        offset: const Offset(50, 50),
        transformHitTests: false,
        child: Transform.translate(
          offset: const Offset(1000, 1000),
          child: Listener(
            onPointerDown: (event) => reported.add(event.localPosition),
            child: const ColoredBox(color: Color(0xFF000000)),
          ),
        ),
      );
      await tester.pumpWidget(scene);
      await tester.tapAt(const Offset(1020, 1010));
      await tester.tapAt(const Offset(1070, 1060));
      expect(reported, [const Offset(20, 10)]);

      reported.clear();
      await tester.pumpWidget(ClipRect(child: scene));
      await tester.tapAt(const Offset(1020, 1010));
      expect(reported, isEmpty);
    },
  );
}
