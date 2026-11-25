import 'dart:ui' show SemanticsHitTestBehavior;

import 'package:flutter/semantics.dart';
import 'package:flutter/widgets.dart' show Rect;
import 'package:flutter_test/flutter_test.dart';

SemanticsNode node(SemanticsHitTestBehavior behavior) => SemanticsNode()
  ..rect = const Rect.fromLTWH(0, 0, 10, 10)
  ..updateWith(
    config: SemanticsConfiguration()
      ..isSemanticBoundary = true
      ..hitTestBehavior = behavior,
  );

void main() {
  test('merged hit-test behavior takes the first override unless parent explicitly sets one', () {
    for (final parent in [
      SemanticsHitTestBehavior.defer,
      SemanticsHitTestBehavior.transparent,
    ]) {
      final root = SemanticsNode()..rect = const Rect.fromLTWH(0, 0, 10, 10);
      root.updateWith(
        config: SemanticsConfiguration()
          ..isSemanticBoundary = true
          ..isMergingSemanticsOfDescendants = true
          ..hitTestBehavior = parent,
        childrenInInversePaintOrder: [
          node(SemanticsHitTestBehavior.defer),
          node(SemanticsHitTestBehavior.opaque),
          node(SemanticsHitTestBehavior.transparent),
        ],
      );
      expect(
        root.getSemanticsData().hitTestBehavior,
        parent == SemanticsHitTestBehavior.defer
            ? SemanticsHitTestBehavior.opaque
            : parent,
      );
    }
  });
}
