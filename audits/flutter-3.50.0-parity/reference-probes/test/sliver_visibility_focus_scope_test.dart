import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final keepSize in [false, true]) {
    testWidgets('retained SliverVisibility focus and scopes: size=$keepSize', (
      tester,
    ) async {
      final focus = FocusNode();
      addTearDown(focus.dispose);
      final controller = ScrollController();
      addTearDown(controller.dispose);
      const reader = ValueKey('reader');
      const next = ValueKey('next');
      final child = SliverToBoxAdapter(
        child: Focus(
          focusNode: focus,
          child: Builder(
            key: reader,
            builder: (context) => SizedBox(
              height: 120,
              child: Text(
                '${Visibility.of(context)} ${TickerMode.of(context)}',
              ),
            ),
          ),
        ),
      );
      Future<void> pump(bool visible) => tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: CustomScrollView(
            controller: controller,
            slivers: [
              SliverVisibility(
                visible: visible,
                maintainState: true,
                maintainAnimation: keepSize,
                maintainSize: keepSize,
                sliver: child,
              ),
              const SliverToBoxAdapter(child: SizedBox(key: next, height: 600)),
            ],
          ),
        ),
      );
      final inner = find.byKey(reader, skipOffstage: false);
      await pump(true);
      final identity = tester.element(inner);
      expect(tester.getTopLeft(find.byKey(next)).dy, 120);
      expect(controller.position.maxScrollExtent, 120);
      focus.requestFocus();
      await tester.pump();
      expect(focus.hasPrimaryFocus, isTrue);

      await pump(false);
      expect(tester.element(inner), same(identity));
      expect(focus.hasPrimaryFocus, isTrue);
      focus.unfocus();
      await tester.pump();
      expect(focus.hasFocus, isFalse);
      focus.requestFocus();
      await tester.pump();
      expect(focus.hasPrimaryFocus, isTrue);
      expect(find.text('true $keepSize', skipOffstage: false), findsOneWidget);
      expect(tester.getTopLeft(find.byKey(next)).dy, keepSize ? 120 : 0);
      expect(controller.position.maxScrollExtent, keepSize ? 120 : 0);

      await pump(true);
      expect(tester.element(inner), same(identity));
      expect(focus.hasPrimaryFocus, isTrue);
      expect(find.text('true true'), findsOneWidget);
      expect(tester.getTopLeft(find.byKey(next)).dy, 120);
      expect(controller.position.maxScrollExtent, 120);
    });
  }
  testWidgets('a default-hidden lazy sliver never builds its children', (
    tester,
  ) async {
    var builds = 0;
    final list = SliverFixedExtentList.builder(
      itemExtent: 100,
      itemCount: 30,
      itemBuilder: (_, index) {
        builds++;
        return Text('Item $index');
      },
    );
    Future<void> pump(bool visible) => tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: CustomScrollView(
          slivers: [SliverVisibility(visible: visible, sliver: list)],
        ),
      ),
    );
    await pump(false);
    expect(builds, 0);
    expect(find.text('Item 0', skipOffstage: false), findsNothing);
    await pump(true);
    expect(builds, greaterThan(0));
    expect(find.text('Item 0'), findsOneWidget);
    final previous = builds;
    await pump(false);
    expect(builds, previous);
    expect(find.text('Item 0', skipOffstage: false), findsNothing);
  });
}
