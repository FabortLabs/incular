import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final String group in <String>['viewport', 'main-axis', 'cross-axis']) {
    testWidgets('Simultaneous sliver fades share the clock in $group', (
      WidgetTester tester,
    ) async {
      final List<int> ended = <int>[0, 0, 0];
      Widget view(List<double> targets) {
        final List<Widget> fades = List<Widget>.generate(
          targets.length,
          (int i) => SliverAnimatedOpacity(
            opacity: targets[i],
            duration: const Duration(milliseconds: 200),
            onEnd: () => ended[i]++,
            sliver: const SliverToBoxAdapter(child: SizedBox(height: 40)),
          ),
        );
        final List<Widget> slivers = switch (group) {
          'viewport' => fades,
          'main-axis' => <Widget>[SliverMainAxisGroup(slivers: fades)],
          _ => <Widget>[SliverCrossAxisGroup(slivers: fades)],
        };
        return Directionality(
          textDirection: TextDirection.ltr,
          child: CustomScrollView(slivers: slivers),
        );
      }

      List<double> alphas() =>
          tester.allRenderObjects
              .whereType<RenderSliverAnimatedOpacity>()
              .toSet()
              .map((RenderSliverAnimatedOpacity render) => render.opacity.value)
              .toList()
            ..sort();

      await tester.pumpWidget(view(<double>[1, 1, 1]));
      await tester.pumpWidget(view(<double>[0.2, 0.4, 0.6]));
      await tester.pump(const Duration(milliseconds: 100));
      expect(
        alphas(),
        orderedEquals(<Matcher>[
          closeTo(0.6, 1e-9),
          closeTo(0.7, 1e-9),
          closeTo(0.8, 1e-9),
        ]),
      );
      expect(ended, <int>[0, 0, 0]);

      await tester.pump(const Duration(milliseconds: 100));
      expect(
        alphas(),
        orderedEquals(<Matcher>[
          closeTo(0.2, 1e-9),
          closeTo(0.4, 1e-9),
          closeTo(0.6, 1e-9),
        ]),
      );
      await tester.pumpAndSettle();
      expect(ended, <int>[1, 1, 1]);
    });
  }
}
