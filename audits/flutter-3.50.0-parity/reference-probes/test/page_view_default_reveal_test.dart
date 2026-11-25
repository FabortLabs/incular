import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('A visible fractional neighbor only reveals when opted in', (
    WidgetTester tester,
  ) async {
    final handle = tester.ensureSemantics();
    final controller = PageController(viewportFraction: 0.625);
    addTearDown(controller.dispose);
    const neighbor = ValueKey<String>('neighbor');
    Widget build({required bool allow}) => Directionality(
      textDirection: TextDirection.ltr,
      child: Align(
        alignment: Alignment.topLeft,
        child: SizedBox(
          width: 400,
          height: 200,
          child: PageView(
            controller: controller,
            allowImplicitScrolling: allow,
            padEnds: false,
            children: <Widget>[
              const SizedBox.expand(),
              Semantics(
                label: 'Adjacent page',
                child: const SizedBox.expand(key: neighbor),
              ),
              const SizedBox.expand(),
            ],
          ),
        ),
      ),
    );
    await tester.pumpWidget(build(allow: false));
    expect(find.bySemanticsLabel('Adjacent page'), findsOneWidget);
    expect(
      tester.getRect(find.byKey(neighbor)),
      const Rect.fromLTWH(250, 0, 250, 200),
    );
    tester.renderObject(find.byKey(neighbor)).showOnScreen();
    await tester.pumpAndSettle();
    expect(controller.position.pixels, 0);

    await tester.pumpWidget(build(allow: true));
    expect(controller.position.pixels, 0);
    final RenderObject child = tester.renderObject(find.byKey(neighbor));
    final RenderAbstractViewport viewport = RenderAbstractViewport.of(child);
    expect(controller.position.physics.allowImplicitScrolling, isTrue);
    final double leading = viewport.getOffsetToReveal(child, 0).offset;
    final double trailing = viewport.getOffsetToReveal(child, 1).offset;
    expect((leading, trailing), (250, 100));
    child.showOnScreen();
    // moveTo jumps immediately, then page physics settles back to page zero.
    expect(controller.position.pixels, 100);
    await tester.pumpAndSettle();
    expect(controller.position.pixels, 0);
    handle.dispose();
  });

  testWidgets('A default PageView passes reveal up to a vertical scroller', (
    WidgetTester tester,
  ) async {
    final inner = PageController(viewportFraction: 0.625);
    final outer = ScrollController();
    addTearDown(inner.dispose);
    addTearDown(outer.dispose);
    const label = ValueKey<String>('nested neighbor');
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Align(
          alignment: Alignment.topLeft,
          child: SizedBox(
            width: 400,
            height: 200,
            child: SingleChildScrollView(
              controller: outer,
              child: Column(
                children: [
                  const SizedBox(height: 100),
                  SizedBox(
                    height: 200,
                    child: PageView(
                      controller: inner,
                      padEnds: false,
                      children: const [
                        SizedBox.expand(),
                        SizedBox.expand(key: label),
                        SizedBox.expand(),
                      ],
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
    expect(
      tester.getRect(find.byKey(label)),
      const Rect.fromLTWH(250, 100, 250, 200),
    );
    tester.renderObject(find.byKey(label)).showOnScreen();
    await tester.pumpAndSettle();
    expect((inner.position.pixels, outer.position.pixels), (0, 100));
  });
}
