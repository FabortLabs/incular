import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final constrained in [false, true]) {
    testWidgets(
      'focused editor retains its state outside a cross-axis lane: $constrained',
      (tester) async {
        final editing = TextEditingController(text: 'draft');
        final focus = FocusNode();
        final scrolling = ScrollController();
        addTearDown(editing.dispose);
        addTearDown(focus.dispose);
        addTearDown(scrolling.dispose);
        const key = ValueKey<String>('field');
        final lane = SliverFixedExtentList.builder(
          itemExtent: 40,
          itemCount: 60,
          itemBuilder: (context, index) => index == 0
              ? EditableText(
                  key: key,
                  controller: editing,
                  focusNode: focus,
                  style: const TextStyle(
                    fontSize: 14,
                    color: Color(0xFF000000),
                  ),
                  cursorColor: const Color(0xFF000000),
                  backgroundCursorColor: const Color(0xFFFFFFFF),
                )
              : const SizedBox(height: 40),
        );
        await tester.pumpWidget(
          Directionality(
            textDirection: TextDirection.ltr,
            child: Align(
              alignment: Alignment.topLeft,
              child: SizedBox(
                width: 300,
                height: 300,
                child: CustomScrollView(
                  controller: scrolling,
                  slivers: [
                    SliverCrossAxisGroup(
                      slivers: [
                        if (constrained)
                          SliverConstrainedCrossAxis(
                            maxExtent: 100,
                            sliver: lane,
                          )
                        else
                          lane,
                        SliverFixedExtentList.builder(
                          itemExtent: 40,
                          itemCount: 60,
                          itemBuilder: (_, _) => const SizedBox(height: 40),
                        ),
                      ],
                    ),
                  ],
                ),
              ),
            ),
          ),
        );
        final mounted = find.byKey(key, skipOffstage: false);
        final editor = tester.state<EditableTextState>(mounted);
        focus.requestFocus();
        await tester.pump();
        expect(focus.hasFocus, isTrue);

        scrolling.jumpTo(1200);
        await tester.pump();
        expect(scrolling.offset, 1200);
        expect(focus.hasFocus, isTrue);
        expect(tester.state<EditableTextState>(mounted), same(editor));
        expect(find.byKey(key).hitTestable(), findsNothing);

        focus.unfocus();
        await tester.pumpAndSettle();
        expect(mounted, findsNothing);
        scrolling.jumpTo(0);
        await tester.pumpAndSettle();
        expect(tester.state<EditableTextState>(mounted), isNot(same(editor)));
        expect(editing.text, 'draft');
      },
    );
  }
}
