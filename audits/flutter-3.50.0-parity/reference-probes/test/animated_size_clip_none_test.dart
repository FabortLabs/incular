import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class PaintLog implements PaintingContext {
  final methods = <Symbol>[];

  @override
  void noSuchMethod(Invocation invocation) {
    methods.add(invocation.memberName);
  }
}

void main() {
  testWidgets('Clip.none paints the overflowing child during growth', (
    tester,
  ) async {
    Widget scene(double size) => Center(
      child: AnimatedSize(
        duration: const Duration(milliseconds: 200),
        clipBehavior: Clip.none,
        child: SizedBox.square(dimension: size),
      ),
    );

    await tester.pumpWidget(scene(100));
    await tester.pumpWidget(scene(200));
    await tester.pump(const Duration(milliseconds: 100));
    final box = tester.renderObject<RenderBox>(find.byType(AnimatedSize));
    expect(box.size, const Size(150, 150));
    final log = PaintLog();
    box.paint(log, Offset.zero);
    expect(log.methods.first, #paintChild);
  });
}
