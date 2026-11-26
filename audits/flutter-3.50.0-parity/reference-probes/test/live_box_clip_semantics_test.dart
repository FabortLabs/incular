import 'package:flutter/semantics.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class AccessibilityClip extends CustomClipper<Rect> {
  AccessibilityClip(this.width) : super(reclip: width);

  final ValueNotifier<double> width;

  @override
  Rect getClip(Size size) => Offset.zero & size;

  @override
  Rect getApproximateClipRect(Size size) =>
      Rect.fromLTWH(0, 0, width.value, size.height);

  @override
  bool shouldReclip(covariant AccessibilityClip oldClipper) =>
      oldClipper.width != width;
}

void main() {
  testWidgets('a live box clip can update semantics independently of hits', (
    tester,
  ) async {
    final semantics = tester.ensureSemantics();
    final width = ValueNotifier<double>(100);
    var presses = 0;
    try {
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Align(
            alignment: Alignment.topLeft,
            child: SizedBox(
              width: 225,
              height: 14,
              child: ClipRect(
                clipper: AccessibilityClip(width),
                child: Row(
                  children: [
                    const SizedBox(width: 75, child: Text('1')),
                    const SizedBox(width: 75, child: Text('2')),
                    SizedBox(
                      width: 75,
                      child: Listener(
                        onPointerDown: (_) => presses += 1,
                        child: const Text('3'),
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ),
        ),
      );

      List<double> widths() {
        final root =
            tester.binding.pipelineOwner.semanticsOwner!.rootSemanticsNode!;
        final observed = <double>[];
        void visit(SemanticsNode node) {
          if (['1', '2', '3'].contains(node.getSemanticsData().label)) {
            observed.add(node.rect.width);
          }
          node.visitChildren((child) {
            visit(child);
            return true;
          });
        }

        visit(root);
        return observed;
      }

      expect(widths(), [75, 25]);
      await tester.tapAt(const Offset(200, 7));
      expect(presses, 1);
      for (final (limit, expected) in <(double, List<double>)>[
        (140, [75, 65]),
        (225, [75, 75, 75]),
        (100, [75, 25]),
      ]) {
        width.value = limit;
        await tester.pump();
        expect(widths(), expected);
        await tester.tapAt(const Offset(200, 7));
      }
      expect(presses, 4);
    } finally {
      await tester.pumpWidget(const SizedBox.shrink());
      width.dispose();
      semantics.dispose();
    }
  });

  testWidgets('a translated semantic node is clipped at the box edge', (
    tester,
  ) async {
    final semantics = tester.ensureSemantics();
    const childKey = ValueKey('shifted child');
    try {
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Center(
            child: ClipRect(
              child: SizedBox(
                width: 40,
                height: 40,
                child: Transform.translate(
                  offset: const Offset(30, 0),
                  child: Semantics(
                    key: childKey,
                    container: true,
                    label: 'shifted child',
                    child: const SizedBox.expand(),
                  ),
                ),
              ),
            ),
          ),
        ),
      );
      final node = tester.getSemantics(find.byKey(childKey));
      expect(node.getSemanticsData().label, 'shifted child');
      expect(node.rect.size, const Size(10, 40));
    } finally {
      semantics.dispose();
    }
  });
}
