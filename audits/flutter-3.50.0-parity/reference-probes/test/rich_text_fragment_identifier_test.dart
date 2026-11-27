import 'dart:ui' as ui;

import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

List<SemanticsNode> parts(WidgetTester tester) {
  final node = tester.getSemantics(find.byType(RichText).first);
  expect(node.label, isEmpty);
  final children = <SemanticsNode>[];
  node.visitChildren((child) {
    children.add(child);
    return true;
  });
  return children;
}

void main() {
  testWidgets('Named rich text ranges interleave inline semantics and retain their IDs', (
    WidgetTester tester,
  ) async {
    Widget paragraph(String last) => Directionality(
      textDirection: TextDirection.ltr,
      child: RichText(
        text: TextSpan(
          text: 'ab',
          locale: const Locale('es', 'MX'),
          children: [
            const WidgetSpan(child: Text('inline widget')),
            TextSpan(
              text: last,
              semanticsLabel: '🦀',
              semanticsIdentifier: 'active',
            ),
          ],
        ),
      ),
    );

    await tester.pumpWidget(paragraph('cd'));
    final children = parts(tester);
    expect(children.map((n) => n.label), ['ab', 'inline widget', '🦀']);
    expect(children.last.identifier, 'active');
    for (final index in [0, 2]) {
      final attribute =
          children[index].attributedLabel.attributes.single as ui.LocaleStringAttribute;
      expect(attribute.locale.toLanguageTag(), 'es-MX');
      expect(attribute.range, const TextRange(start: 0, end: 2));
    }
    final ids = children.map((n) => n.id).toList();

    await tester.pumpWidget(paragraph('ef'));
    expect(parts(tester).map((n) => n.id), ids);
  });

  testWidgets('Spoken alias length does not move named ranges or expose ellipsized children', (
    WidgetTester tester,
  ) async {
    Future<void> mount(double width, String alias) async {
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Align(
            alignment: Alignment.topLeft,
            child: SizedBox(
              width: width,
              child: RichText(
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                text: TextSpan(
                  text: 'AB',
                  style: const TextStyle(fontSize: 10),
                  semanticsLabel: alias,
                  spellOut: true,
                  children: [
                    WidgetSpan(
                      child: Semantics(
                        container: true,
                        label: 'inline widget',
                        child: const SizedBox(width: 60, height: 10),
                      ),
                    ),
                    const TextSpan(text: 'CD', semanticsIdentifier: 'named'),
                  ],
                ),
              ),
            ),
          ),
        ),
      );
    }

    ui.Rect namedBounds() {
      final children = parts(tester);
      expect(children.map((node) => node.label).skip(1), ['inline widget', 'CD']);
      final named = children.last;
      expect(named.identifier, 'named');
      expect(
        children.first.attributedLabel.attributes.single.range,
        TextRange(start: 0, end: children.first.label.length),
      );
      final paragraph = tester.renderObject<RenderParagraph>(find.byType(RichText));
      final displayBox = paragraph.getBoxesForSelection(
        const TextSelection(baseOffset: 3, extentOffset: 5),
      ).single;
      expect(named.rect.left, displayBox.left.floorToDouble() - 4);
      expect(named.rect.right, displayBox.right.ceilToDouble() + 4);
      return named.rect;
    }

    await mount(120, 'alpha 🦀 beta');
    final originalBounds = namedBounds();
    await mount(120, 'an even longer spoken label');
    expect(namedBounds(), originalBounds);
    await mount(94, 'an even longer spoken label');
    expect(parts(tester).map((node) => node.label), [
      'an even longer spoken label',
      'inline widget',
    ]);
    await mount(36, 'an even longer spoken label');
    expect(parts(tester).map((node) => node.identifier), everyElement(isEmpty));
    expect(parts(tester).map((node) => node.label), isNot(contains('inline widget')));
    await mount(120, 'an even longer spoken label');
    expect(namedBounds(), originalBounds);
  });
}
