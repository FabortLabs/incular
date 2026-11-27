import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  const child = AspectRatio(
    aspectRatio: 2,
    child: SizedBox(width: 100, height: 40),
  );

  testWidgets('unbounded queries derive a finite perpendicular extent', (
    tester,
  ) async {
    await tester.pumpWidget(
      const Center(
        child: IntrinsicWidth(stepWidth: 47, stepHeight: 10, child: child),
      ),
    );
    final RenderBox width = tester.renderObject(find.byType(IntrinsicWidth));
    expect(width.getMinIntrinsicHeight(double.infinity), 80);
    expect(width.getMaxIntrinsicHeight(double.infinity), 80);
    expect(width.getMaxIntrinsicHeight(120), 60);

    await tester.pumpWidget(const Center(child: IntrinsicHeight(child: child)));
    final RenderBox height = tester.renderObject(find.byType(IntrinsicHeight));
    expect(height.getMinIntrinsicWidth(double.infinity), 80);
    expect(height.getMaxIntrinsicWidth(double.infinity), 80);
    expect(height.getMaxIntrinsicWidth(30), 60);
  });
}
