import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'BlockSemantics propagates through padding but stops at a semantic boundary',
    (t) async {
      final handle = t.ensureSemantics();
      Widget tag(String s) => Semantics(
        container: true,
        label: s,
        child: const SizedBox(width: 40, height: 40),
      );
      Future<void> check(
        Widget middle, {
        required bool early,
        bool boundary = false,
      }) async {
        await t.pumpWidget(
          Directionality(
            textDirection: TextDirection.ltr,
            child: Stack(children: [tag('earlier'), middle, tag('later')]),
          ),
        );
        final labels = <String>{};
        void walk(SemanticsNode node) {
          labels.add(node.getSemanticsData().label);
          node.visitChildren((child) {
            walk(child);
            return true;
          });
        }

        walk(
          t
              .binding
              .renderViews
              .single
              .owner!
              .semanticsOwner!
              .rootSemanticsNode!,
        );
        expect(labels.contains('earlier'), early, reason: labels.toString());
        expect(labels, containsAll(['front', 'later']));
        expect(labels.contains('boundary'), boundary);
        expect(labels, isNot(contains('inside earlier')));
      }

      try {
        await check(BlockSemantics(child: tag('front')), early: false);
        await check(
          Semantics(
            container: true,
            explicitChildNodes: true,
            label: 'boundary',
            child: Stack(
              children: [
                tag('inside earlier'),
                BlockSemantics(child: tag('front')),
              ],
            ),
          ),
          early: true,
          boundary: true,
        );
        await check(
          Padding(
            padding: const EdgeInsets.all(3),
            child: BlockSemantics(child: tag('front')),
          ),
          early: false,
        );
      } finally {
        handle.dispose();
      }
    },
  );
}
