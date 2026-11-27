import 'dart:ui' show SemanticsAction;

import 'package:flutter/rendering.dart';
import 'package:flutter/semantics.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'a list viewport tag is consumed before a nested ordinary scroller',
    (tester) async {
      final SemanticsHandle handle = tester.ensureSemantics();
      try {
        await tester.pumpWidget(
          const Directionality(
            textDirection: TextDirection.ltr,
            child: _Scene(),
          ),
        );
        Iterable<SemanticsNode> descendants(SemanticsNode node) sync* {
          yield node;
          for (final SemanticsNode child in node.debugListChildrenInOrder(
            DebugSemanticsDumpOrder.traversalOrder,
          )) {
            yield* descendants(child);
          }
        }

        int? stable;
        for (var frame = 0; frame < 2; frame += 1) {
          final SemanticsNode root =
              tester.binding.pipelineOwner.semanticsOwner!.rootSemanticsNode!;
          final all = descendants(root).toList();
          final SemanticsNode index = all
              .where(
                (node) => node.isTagged(RenderViewport.useTwoPaneSemantics),
              )
              .single;
          expect(index.indexInParent, 0);
          expect(index.flagsCollection.hasImplicitScrolling, isFalse);
          final SemanticsNode scroller = index
              .debugListChildrenInOrder(DebugSemanticsDumpOrder.traversalOrder)
              .single;
          expect(scroller.flagsCollection.hasImplicitScrolling, isTrue);
          expect(
            scroller.isTagged(RenderViewport.useTwoPaneSemantics),
            isFalse,
          );
          expect(scroller.indexInParent, isNull);
          expect(
            scroller
                .debugListChildrenInOrder(
                  DebugSemanticsDumpOrder.traversalOrder,
                )
                .single
                .label,
            'label',
          );
          expect(index.id, stable ??= index.id);
          await tester.pump();
        }
      } finally {
        handle.dispose();
      }
    },
    semanticsEnabled: false,
  );
  testWidgets('point actions reach content through semantics-only pane nodes', (
    tester,
  ) async {
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.resetDevicePixelRatio);
    var taps = 0;
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: _Scene(
          onTap: () {
            taps += 1;
          },
        ),
      ),
    );
    final SemanticsOwner owner = tester.binding.pipelineOwner.semanticsOwner!;
    owner.performActionAt(
      tester.getCenter(find.text('label')),
      SemanticsAction.tap,
    );
    expect(taps, 1);
  });
}

class _Scene extends StatelessWidget {
  const _Scene({this.onTap});

  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) => ListView(
    children: [
      SingleChildScrollView(
        child: Column(
          children: [
            const SizedBox(height: 20),
            if (onTap == null)
              const Text('label')
            else
              Semantics(
                container: true,
                onTap: onTap,
                child: const Text('label'),
              ),
          ],
        ),
      ),
    ],
  );
}
