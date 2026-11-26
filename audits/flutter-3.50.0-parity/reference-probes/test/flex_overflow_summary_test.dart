import 'package:flutter/foundation.dart' show DiagnosticLevel, FlutterError;
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Flex reports the Flutter debug overflow summary at paint', (
    tester,
  ) async {
    Widget scene(
      Axis axis, {
      required Key key,
      double extent = 225,
      bool offstage = false,
      double budget = 100,
    }) {
      final horizontal = axis == Axis.horizontal;
      return Directionality(
        textDirection: TextDirection.rtl,
        child: Align(
          alignment: Alignment.topLeft,
          child: Offstage(
            offstage: offstage,
            child: SizedBox(
              width: horizontal ? budget : 14,
              height: horizontal ? 14 : budget,
              child: Flex(
                key: key,
                direction: axis,
                children: [
                  SizedBox(
                    width: horizontal ? extent : 14,
                    height: horizontal ? 14 : extent,
                  ),
                ],
              ),
            ),
          ),
        ),
      );
    }

    void expectSummary(String summary) {
      final error = tester.takeException()! as FlutterError;
      expect(error.diagnostics.first.level, DiagnosticLevel.summary);
      expect(error.diagnostics.first.toString(), summary);
    }

    const verticalKey = ValueKey('vertical');
    await tester.pumpWidget(
      scene(Axis.vertical, key: verticalKey, offstage: true),
    );
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(scene(Axis.vertical, key: verticalKey));
    expectSummary('A RenderFlex overflowed by 125 pixels on the bottom.');
    await tester.pumpWidget(
      scene(Axis.vertical, key: verticalKey, budget: 110),
    );
    expect(tester.takeException(), isNull);

    for (final (i, pixels, printed) in [
      (0, 125.0, '125'),
      (1, 12.5, '13'),
      (2, 0.3125, '0.313'),
    ]) {
      await tester.pumpWidget(
        scene(Axis.horizontal, key: ValueKey(i), extent: 100 + pixels),
      );
      expectSummary('A RenderFlex overflowed by $printed pixels on the right.');
    }
  });
}
