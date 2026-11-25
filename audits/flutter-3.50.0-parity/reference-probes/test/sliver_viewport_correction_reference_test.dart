import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _Correction extends SingleChildRenderObjectWidget {
  const _Correction(this.attempted, {required super.child});
  final List<double> attempted;

  @override
  RenderSliverToBoxAdapter createRenderObject(BuildContext context) =>
      _RenderCorrection(attempted);
}

class _RenderCorrection extends RenderSliverToBoxAdapter {
  _RenderCorrection(this.attempted);
  final List<double> attempted;

  @override
  void performLayout() {
    attempted.add(constraints.scrollOffset);
    if (constraints.scrollOffset != 200) {
      geometry = SliverGeometry(
        scrollOffsetCorrection: 200 - constraints.scrollOffset,
      );
      return;
    }
    super.performLayout();
  }
}

void main() {
  testWidgets('viewport lays out the corrected offset before the first paint', (
    tester,
  ) async {
    final attempted = <double>[];
    final controller = ScrollController();
    addTearDown(controller.dispose);
    const child = ValueKey('corrected');
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: CustomScrollView(
          controller: controller,
          slivers: [
            SliverPadding(
              padding: EdgeInsets.zero,
              sliver: _Correction(
                attempted,
                child: const SizedBox(key: child, height: 400),
              ),
            ),
            const SliverToBoxAdapter(child: SizedBox(height: 600)),
          ],
        ),
      ),
    );
    expect(controller.offset, 200);
    expect(attempted.first, 0);
    expect(attempted.skip(1), everyElement(200));
    expect(tester.getTopLeft(find.byKey(child)), const Offset(0, -200));

    attempted.clear();
    controller.jumpTo(350);
    await tester.pump();
    expect(controller.offset, 200);
    expect(attempted.first, 350);
    expect(attempted.skip(1), everyElement(200));
    expect(tester.getTopLeft(find.byKey(child)), const Offset(0, -200));
  });
}
