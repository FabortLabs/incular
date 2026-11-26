import 'dart:ui' as ui;

import 'package:flutter/semantics.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Single values and hints merge until a second value conflicts', (
    tester,
  ) async {
    const rootKey = ValueKey('root');
    const boundaryKey = ValueKey('boundary');
    Widget scene(bool twoValues) => Directionality(
      textDirection: TextDirection.ltr,
      child: Semantics(
        key: rootKey,
        container: true,
        child: Column(
          children: [
            Semantics(
              hint: 'first hint',
              value: 'first',
              child: const SizedBox.square(dimension: 10),
            ),
            Semantics(
              key: boundaryKey,
              container: true,
              hint: 'separate',
              child: const SizedBox.square(dimension: 10),
            ),
            Semantics(
              hint: 'second hint',
              value: twoValues ? 'second' : null,
              child: const SizedBox.square(dimension: 10),
            ),
          ],
        ),
      ),
    );
    await tester.pumpWidget(scene(false));
    final rootId = tester.getSemantics(find.byKey(rootKey)).id;
    final boundaryId = tester.getSemantics(find.byKey(boundaryKey)).id;
    for (final twoValues in [false, true, false]) {
      await tester.pumpWidget(scene(twoValues));
      final node = tester.getSemantics(find.byKey(rootKey));
      expect(node.id, rootId);
      expect(node.hint, twoValues ? '' : 'first hint\nsecond hint');
      expect(node.value, twoValues ? '' : 'first');
      final children = <(String, String)>[];
      node.visitChildren((child) {
        children.add((child.hint, child.value));
        return true;
      });
      expect(
        children,
        twoValues
            ? [
                ('first hint', 'first'),
                ('separate', ''),
                ('second hint', 'second'),
              ]
            : [('separate', '')],
      );
      expect(tester.getSemantics(find.byKey(boundaryKey)).id, boundaryId);
    }
  });

  testWidgets('Merged hints retain UTF-16 spans and opposite direction marks', (
    tester,
  ) async {
    const key = ValueKey('root');
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Semantics(
          key: key,
          container: true,
          child: Column(
            children: [
              Semantics(
                hint: '😀',
                child: const SizedBox.square(dimension: 10),
              ),
              Semantics(
                attributedHint: AttributedString(
                  'אב',
                  attributes: [
                    ui.SpellOutStringAttribute(
                      range: const ui.TextRange(start: 0, end: 2),
                    ),
                  ],
                ),
                textDirection: TextDirection.rtl,
                child: const SizedBox.square(dimension: 10),
              ),
              Semantics(
                attributedValue: AttributedString(
                  'five',
                  attributes: [
                    ui.SpellOutStringAttribute(
                      range: const ui.TextRange(start: 0, end: 4),
                    ),
                  ],
                ),
                child: const SizedBox.square(dimension: 10),
              ),
            ],
          ),
        ),
      ),
    );
    final node = tester.getSemantics(find.byKey(key));
    expect(node.textDirection, TextDirection.ltr);
    expect(node.hint, '😀\n\u202bאב\u202c');
    expect(
      node.attributedHint.attributes.single.range,
      const ui.TextRange(start: 4, end: 6),
    );
    expect(node.value, 'five');
    expect(
      node.attributedValue.attributes.single.range,
      const ui.TextRange(start: 0, end: 4),
    );
    expect(node.childrenCount, 0);
  });
}
