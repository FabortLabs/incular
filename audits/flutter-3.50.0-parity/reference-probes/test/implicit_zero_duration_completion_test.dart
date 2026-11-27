import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final sliver in [false, true]) {
    testWidgets('zero-duration opacity reports completion once (sliver=$sliver)', (tester) async {
      final completions = <String>[];
      Widget view(double target, String label) {
        void end() => completions.add(label);
        return Directionality(
          textDirection: TextDirection.ltr,
          child: sliver
            ? CustomScrollView(slivers: [
                SliverAnimatedOpacity(
                  opacity: target,
                  duration: Duration.zero,
                  onEnd: end,
                  sliver: const SliverToBoxAdapter(child: SizedBox(height: 50)),
                ),
              ])
            : AnimatedOpacity(
                opacity: target,
                duration: Duration.zero,
                onEnd: end,
                child: const SizedBox(height: 50),
              ),
        );
      }

      double alpha() => sliver
        ? tester.widget<SliverFadeTransition>(find.byType(SliverFadeTransition)).opacity.value
        : tester.widget<FadeTransition>(find.byType(FadeTransition)).opacity.value;

      await tester.pumpWidget(view(1, 'initial'));
      expect(alpha(), 1);
      expect(completions, isEmpty);

      await tester.pumpWidget(view(0, 'zero'));
      expect(alpha(), 0);
      expect(completions, ['zero']);

      await tester.pumpWidget(view(0, 'unchanged'));
      await tester.pump(const Duration(milliseconds: 16));
      expect(completions, ['zero']);

      await tester.pumpWidget(view(0.5, 'latest'));
      expect(alpha(), 0.5);
      expect(completions, ['zero', 'latest']);
      await tester.pump(const Duration(milliseconds: 16));
      expect(completions, ['zero', 'latest']);
    });
  }
}
