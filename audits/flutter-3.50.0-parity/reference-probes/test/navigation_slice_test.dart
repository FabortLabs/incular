import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class ProbePage extends Page<void> {
  ProbePage(this.child, {super.key}) : super(name: 'editor');
  final Widget child;
  @override
  Route<void> createRoute(BuildContext ctx) => PageRouteBuilder<void>(
    settings: this,
    transitionDuration: Duration.zero,
    reverseTransitionDuration: Duration.zero,
    pageBuilder: (_, __, ___) => child,
  );
}

class TrackedPage extends Page<void> {
  TrackedPage(String name, {super.key}) : super(name: name);

  Route<void>? created;

  @override
  Route<void> createRoute(BuildContext context) => created = PageRouteBuilder<void>(
    settings: this,
    transitionDuration: Duration.zero,
    reverseTransitionDuration: Duration.zero,
    pageBuilder: (_, __, ___) => Text(name!),
  );
}

class Counter extends StatefulWidget {
  const Counter({super.key});
  @override
  State<Counter> createState() => CounterState();
}

class CounterState extends State<Counter> {
  int count = 0;
  void bump() => setState(() => count++);
  @override
  Widget build(BuildContext context) => Text('count $count');
}

void main() {
  for (final keyed in [true, false]) {
    testWidgets(
      'compatible page preserves route and child state: keyed=$keyed',
      (t) async {
        final key = keyed ? const ValueKey('editor') : null;
        final first = ProbePage(Counter(), key: key);
        final second = ProbePage(Counter(), key: key);
        Widget shell(ProbePage page) => MediaQuery(
          data: MediaQueryData.fromView(t.view),
          child: Directionality(
            textDirection: TextDirection.ltr,
            child: Navigator(pages: [page], onDidRemovePage: (_) {}),
          ),
        );
        final child = find.byType(Counter);
        await t.pumpWidget(shell(first));
        final beforeState = t.state<CounterState>(child);
        final beforeRoute = ModalRoute.of<void>(t.element(child))!;
        beforeState.bump();
        await t.pump();
        expect(find.text('count 1'), findsOneWidget);
        expect(identical(first, second), isFalse);
        await t.pumpWidget(shell(second));
        final afterRoute = ModalRoute.of<void>(t.element(child))!;
        expect(afterRoute, same(beforeRoute));
        expect(afterRoute.settings, same(second));
        expect(t.state<CounterState>(child), same(beforeState));
        expect(find.text('count 1'), findsOneWidget);
      },
    );
  }
  testWidgets('unkeyed pages match only at compatible list edges', (t) async {
    TrackedPage keyed(String name) => TrackedPage(name, key: ValueKey(name));
    final old = [
      TrackedPage('bottom'),
      keyed('a'),
      TrackedPage('middle'),
      keyed('b'),
      TrackedPage('top'),
    ];
    final next = [
      TrackedPage('new bottom'),
      keyed('b'),
      TrackedPage('middle'),
      keyed('a'),
      TrackedPage('new top'),
    ];
    Widget shell(List<TrackedPage> pages) => MediaQuery(
      data: MediaQueryData.fromView(t.view),
      child: Directionality(
        textDirection: TextDirection.ltr,
        child: Navigator(pages: pages, onDidRemovePage: (_) {}),
      ),
    );
    await t.pumpWidget(shell(old));
    final routes = old.map((page) => page.created!).toList();
    await t.pumpWidget(shell(next));
    await t.pumpAndSettle();
    for (final (index, routeIndex) in [(0, 0), (1, 3), (3, 1), (4, 4)]) {
      expect(next[index].created, isNull);
      expect(routes[routeIndex].settings, same(next[index]));
    }
    expect(next[2].created, isNotNull);
    expect(next[2].created, isNot(same(routes[2])));
    expect(routes[2].isActive, isFalse);
    expect(routes[4].isCurrent, isTrue);
  });
  testWidgets('pageless route stays attached to a retained page', (t) async {
    final nav = GlobalKey<NavigatorState>();
    Widget shell() => MediaQuery(
      data: MediaQueryData.fromView(t.view),
      child: Directionality(
        textDirection: TextDirection.ltr,
        child: Navigator(
          key: nav,
          pages: [ProbePage(Counter(), key: const ValueKey('editor'))],
          onDidRemovePage: (_) {},
        ),
      ),
    );
    await t.pumpWidget(shell());
    final pushed = PageRouteBuilder<void>(
      transitionDuration: Duration.zero,
      reverseTransitionDuration: Duration.zero,
      pageBuilder: (_, __, ___) => const Text('imperative'),
    );
    nav.currentState!.push<void>(pushed);
    await t.pumpAndSettle();
    expect(find.text('imperative'), findsOneWidget);
    await t.pumpWidget(shell());
    await t.pumpAndSettle();
    expect(find.text('imperative'), findsOneWidget);
    expect(pushed.isCurrent, isTrue);
  });
}
