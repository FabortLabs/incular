import 'dart:ui' show SemanticsRole;

import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'A tab may omit tap only while disabled or user actions are blocked',
    (tester) async {
      Widget tab({bool? enabled, bool blocked = false}) {
        return Directionality(
          textDirection: TextDirection.ltr,
          child: Semantics(
            role: SemanticsRole.tab,
            selected: false,
            enabled: enabled,
            blockUserActions: blocked,
            child: const Text('a tab'),
          ),
        );
      }

      await tester.pumpWidget(tab(enabled: false));
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(tab(blocked: true));
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(tab());
      expect(
        tester.takeException(),
        isFlutterError.having(
          (e) => e.message,
          'message',
          'A tab must have a tap action',
        ),
      );
    },
  );
}
