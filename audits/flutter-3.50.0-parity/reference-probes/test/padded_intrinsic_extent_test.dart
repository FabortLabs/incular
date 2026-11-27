import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _MeasuredBox extends LeafRenderObjectWidget {
  const _MeasuredBox();

  @override
  RenderBox createRenderObject(BuildContext context) => _MeasuredRenderBox();
}

class _MeasuredRenderBox extends RenderBox {
  @override
  double computeMaxIntrinsicWidth(double height) => 100.0;

  @override
  double computeMaxIntrinsicHeight(double width) => 200.0;

  @override
  void performLayout() => size = constraints.constrain(const Size(55, 110));
}

void main() {
  const padded = Padding(
    padding: EdgeInsets.symmetric(horizontal: 10, vertical: 15),
    child: _MeasuredBox(),
  );

  testWidgets('padding adds its horizontal extent to the child maximum width', (
    tester,
  ) async {
    await tester.pumpWidget(const Center(child: IntrinsicWidth(child: padded)));
    expect(tester.getSize(find.byType(IntrinsicWidth)), const Size(120, 140));
    expect(tester.getSize(find.byType(Padding)), const Size(120, 140));
    expect(tester.getSize(find.byType(_MeasuredBox)), const Size(100, 110));
  });

  testWidgets('padding adds its vertical extent to the child maximum height', (
    tester,
  ) async {
    await tester.pumpWidget(
      const Center(child: IntrinsicHeight(child: padded)),
    );
    expect(tester.getSize(find.byType(IntrinsicHeight)), const Size(75, 230));
    expect(tester.getSize(find.byType(Padding)), const Size(75, 230));
    expect(tester.getSize(find.byType(_MeasuredBox)), const Size(55, 200));
  });
}
