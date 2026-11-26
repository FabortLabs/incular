import 'package:flutter/semantics.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('retained Flex semantic rectangles follow the active clip', (
    tester,
  ) async {
    final semantics = tester.ensureSemantics();

    try {
      final scenarios = <(double, Clip, List<double>)>[
        (100, Clip.hardEdge, [75, 25]),
        (130, Clip.hardEdge, [75, 55]),
        (150, Clip.hardEdge, [75, 75]),
        (225, Clip.hardEdge, [75, 75, 75]),
        (100, Clip.hardEdge, [75, 25]),
        (100, Clip.none, [75, 75, 75]),
        (100, Clip.hardEdge, [75, 25]),
      ];
      int? firstId;
      for (final (width, clip, expected) in scenarios) {
        await tester.pumpWidget(
          Directionality(
            textDirection: TextDirection.ltr,
            child: Align(
              alignment: Alignment.topLeft,
              child: SizedBox(
                width: width,
                height: 14,
                child: Flex(
                  clipBehavior: clip,
                  direction: Axis.horizontal,
                  children: [
                    for (final label in ['1', '2', '3'])
                      SizedBox(width: 75, child: Text(label)),
                  ],
                ),
              ),
            ),
          ),
        );
        final overflow = tester.takeException();
        if (overflow != null) {
          expect(overflow.toString(), contains('overflowed'));
        }
        final root =
            tester.binding.pipelineOwner.semanticsOwner!.rootSemanticsNode!;
        final labelled = <SemanticsNode>[];
        void visit(SemanticsNode node) {
          if (['1', '2', '3'].contains(node.getSemanticsData().label)) {
            labelled.add(node);
          }
          node.visitChildren((child) {
            visit(child);
            return true;
          });
        }

        visit(root);
        expect(
          labelled.map((node) => node.rect.width),
          expected,
          reason: 'width $width, clip $clip',
        );
        firstId ??= labelled.first.id;
        expect(labelled.first.id, firstId);
      }
    } finally {
      semantics.dispose();
    }
  });
}
