import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'A sliver boundary can absorb and release the current text label',
    (WidgetTester tester) async {
      const ValueKey<String> key = ValueKey<String>('sliver boundary');
      int? retained;
      for (final (String label, bool explicit) in <(String, bool)>[
        ('initial', false),
        ('changed', false),
        ('standalone', true),
        ('absorbed', false),
      ]) {
        await tester.pumpWidget(
          Directionality(
            textDirection: TextDirection.ltr,
            child: CustomScrollView(
              slivers: <Widget>[
                SliverSemantics(
                  key: key,
                  container: true,
                  explicitChildNodes: explicit,
                  sliver: SliverToBoxAdapter(child: Text(label)),
                ),
              ],
            ),
          ),
        );
        final SemanticsNode node = tester.getSemantics(find.byKey(key));
        retained ??= node.id;
        expect(node.id, retained);
        final List<SemanticsNode> children = <SemanticsNode>[];
        node.visitChildren((SemanticsNode child) {
          children.add(child);
          return true;
        });
        if (explicit) {
          expect(node.getSemanticsData().label, isEmpty);
          expect(children.single.getSemanticsData().label, label);
        } else {
          expect(node.getSemanticsData().label, label);
          expect(children, isEmpty);
        }
      }
    },
  );
}
