import 'package:flutter/semantics.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _Policy extends CustomPainter {
  _Policy(
    this.label,
    this.paintChanges,
    this.semanticsChange,
    this.paints,
    this.regions,
  );
  final String label;
  final bool paintChanges;
  final bool semanticsChange;
  final List<String> paints;
  final List<Size> regions;

  @override
  void paint(Canvas canvas, Size size) => paints.add(label);

  @override
  bool shouldRepaint(_Policy oldDelegate) => paintChanges;

  @override
  bool shouldRebuildSemantics(_Policy oldDelegate) => semanticsChange;

  @override
  SemanticsBuilderCallback get semanticsBuilder => (size) {
    regions.add(size);
    return [
      CustomPainterSemantics(
        key: const ValueKey('retained'),
        rect: Rect.fromLTWH(0, 0, size.width / 2, 20),
        properties: SemanticsProperties(
          label: label,
          textDirection: TextDirection.ltr,
        ),
      ),
    ];
  };
}

void main() {
  testWidgets('paint and semantic reconstruction have independent gates', (
    tester,
  ) async {
    final semantics = tester.ensureSemantics();
    final paints = <String>[];
    final regions = <Size>[];
    Future<void> show(
      String label,
      double width,
      bool paintChanges,
      bool semanticChanges,
    ) {
      return tester.pumpWidget(
        Align(
          alignment: Alignment.topLeft,
          child: CustomPaint(
            size: Size(width, 40),
            painter: _Policy(
              label,
              paintChanges,
              semanticChanges,
              paints,
              regions,
            ),
          ),
        ),
      );
    }

    SemanticsNode locate(String label) {
      SemanticsNode? found;
      void walk(SemanticsNode node) {
        if (node.getSemanticsData().label == label) {
          found = node;
        }
        node.visitChildren((child) {
          walk(child);
          return true;
        });
      }

      walk(
        tester
            .binding
            .renderViews
            .single
            .owner!
            .semanticsOwner!
            .rootSemanticsNode!,
      );
      return found!;
    }

    await show('old', 100, true, false);
    final id = locate('old').id;
    expect(regions, [const Size(100, 40)]);
    await show('pending', 100, true, false);
    expect(locate('old').id, id);
    expect(paints.last, 'pending');
    expect(regions, [const Size(100, 40)]);

    await show('pending', 120, true, false);
    expect(locate('pending').id, id);
    expect(locate('pending').rect.size, const Size(60, 20));
    expect(regions, [const Size(100, 40), const Size(120, 40)]);

    final paintCount = paints.length;
    await show('fresh', 120, false, true);
    expect(locate('fresh').id, id);
    expect(paints.length, paintCount);
    expect(regions, [
      const Size(100, 40),
      const Size(120, 40),
      const Size(120, 40),
    ]);
    semantics.dispose();
  });
}
