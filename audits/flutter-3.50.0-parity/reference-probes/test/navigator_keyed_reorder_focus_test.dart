import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class FocusPage extends Page<void> {
  FocusPage(String name, this.node) : super(name: name, key: ValueKey(name));

  final FocusNode node;

  @override
  Route<void> createRoute(BuildContext context) => PageRouteBuilder<void>(
    settings: this,
    transitionDuration: Duration.zero,
    reverseTransitionDuration: Duration.zero,
    pageBuilder: (_, _, _) => Focus(
      focusNode: node,
      child: SizedBox.expand(key: ValueKey('content-$name')),
    ),
  );
}

void main() {
  testWidgets('keyed reordering keeps mounted focus histories separate', (tester) async {
    final navigatorKey = GlobalKey<NavigatorState>();
    final a = FocusNode();
    final b = FocusNode();
    final newB = FocusNode();
    addTearDown(a.dispose);
    addTearDown(b.dispose);
    addTearDown(newB.dispose);

    Future<void> present(List<FocusPage> pages) async {
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Navigator(
            key: navigatorKey,
            pages: pages,
            onDidRemovePage: (_) {},
          ),
        ),
      );
      await tester.pumpAndSettle();
    }

    await present([FocusPage('a', a), FocusPage('b', b)]);
    final originalB = ModalRoute.of(b.context!);
    b.requestFocus();
    await tester.pumpAndSettle();
    expect(b.hasPrimaryFocus, isTrue);

    await present([FocusPage('b', b), FocusPage('a', a)]);
    expect(ModalRoute.of(b.context!), same(originalB));
    // Reordering two existing routes is not a push. Flutter leaves the
    // prior primary focus in place until an explicit request takes it.
    expect(b.hasPrimaryFocus, isTrue);
    await present([FocusPage('b', b), FocusPage('a', a)]);
    expect(b.hasPrimaryFocus, isTrue);
    a.requestFocus();
    await tester.pumpAndSettle();
    expect(a.hasPrimaryFocus, isTrue);

    await present([FocusPage('a', a), FocusPage('b', b)]);
    expect(ModalRoute.of(b.context!), same(originalB));
    expect(a.hasPrimaryFocus, isTrue);
    expect(b.hasPrimaryFocus, isFalse);

    await present([FocusPage('a', a)]);
    expect(find.byKey(const ValueKey('content-b'), skipOffstage: false), findsNothing);
    await present([FocusPage('a', a), FocusPage('b', newB)]);
    expect(ModalRoute.of(newB.context!), isNot(same(originalB)));
    expect(newB.hasPrimaryFocus, isFalse);
  });
}
