import 'package:flutter/semantics.dart';
import 'package:flutter/widgets.dart' show Rect;
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('the display name does not identify an allocated semantics tag', () {
    final a = SemanticsTag('same');
    final b = SemanticsTag('same');
    final node = SemanticsNode()..tags = <SemanticsTag>{a};
    expect(node.isTagged(a), isTrue);
    expect(node.isTagged(b), isFalse);
  });

  test('merged semantics uses the union of disjoint or overlapping tags', () {
    const first = SemanticsTag('First');
    const second = SemanticsTag('Second');
    const third = SemanticsTag('Third');
    final parent = SemanticsNode()
      ..rect = const Rect.fromLTWH(0, 0, 10, 10)
      ..tags = <SemanticsTag>{first, second};
    final child = SemanticsNode()
      ..rect = const Rect.fromLTWH(1, 1, 5, 5)
      ..tags = <SemanticsTag>{second, third};
    final config = SemanticsConfiguration()
      ..isSemanticBoundary = true
      ..isMergingSemanticsOfDescendants = true;
    parent.updateWith(config: config, childrenInInversePaintOrder: [child]);
    expect(parent.getSemanticsData().tags, <SemanticsTag>{
      first,
      second,
      third,
    });
  });
}
