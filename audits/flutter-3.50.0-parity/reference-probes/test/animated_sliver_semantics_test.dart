import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final bool implicit in <bool>[false, true]) {
    testWidgets(
      '${implicit ? 'implicit' : 'explicit'} sliver opacity republishes semantics without drawing',
      (WidgetTester tester) async {
        final SemanticsHandle semantics = tester.ensureSemantics();
        final GlobalKey key = GlobalKey();
        const String label = 'animated child';

        bool published(SemanticsNode node) {
          if (node.label == label) {
            return true;
          }
          bool found = false;
          node.visitChildren((SemanticsNode child) {
            found = published(child);
            return !found;
          });
          return found;
        }

        bool hasLabel() => published(
          tester
              .binding
              .renderViews
              .single
              .owner!
              .semanticsOwner!
              .rootSemanticsNode!,
        );

        Future<void> layout({required double opacity, bool? include}) {
          final Widget sliver = SliverToBoxAdapter(
            child: Semantics(
              key: key,
              label: label,
              container: true,
              child: const SizedBox(
                height: 40,
                child: ColoredBox(color: Color(0xFFFFFFFF)),
              ),
            ),
          );
          final Widget fade = implicit
              ? include == null
                    ? SliverAnimatedOpacity(
                        opacity: opacity,
                        duration: const Duration(milliseconds: 100),
                        sliver: sliver,
                      )
                    : SliverAnimatedOpacity(
                        opacity: opacity,
                        duration: const Duration(milliseconds: 100),
                        sliver: sliver,
                        alwaysIncludeSemantics: include,
                      )
              : include == null
              ? SliverFadeTransition(
                  opacity: AlwaysStoppedAnimation<double>(opacity),
                  sliver: sliver,
                )
              : SliverFadeTransition(
                  opacity: AlwaysStoppedAnimation<double>(opacity),
                  sliver: sliver,
                  alwaysIncludeSemantics: include,
                );
          return tester.pumpWidget(
            Directionality(
              textDirection: TextDirection.ltr,
              child: CustomScrollView(slivers: <Widget>[fade]),
            ),
          );
        }

        await layout(opacity: 0);
        final BuildContext? mounted = key.currentContext;
        expect(mounted, isNotNull);
        final RenderSliverAnimatedOpacity opacitySliver = tester.renderObject(
          find.byType(SliverFadeTransition),
        );
        expect(opacitySliver.paintsChild(opacitySliver.child!), isFalse);
        expect(hasLabel(), isFalse);
        await layout(opacity: 0, include: true);
        expect(hasLabel(), isTrue);
        expect(opacitySliver.paintsChild(opacitySliver.child!), isFalse);
        await layout(opacity: 0, include: false);
        expect(hasLabel(), isFalse);
        await layout(opacity: 0.5, include: false);
        await tester.pumpAndSettle();
        expect(hasLabel(), isTrue);
        expect(opacitySliver.paintsChild(opacitySliver.child!), isTrue);
        expect(key.currentContext, same(mounted));
        semantics.dispose();
      },
    );
  }
}
