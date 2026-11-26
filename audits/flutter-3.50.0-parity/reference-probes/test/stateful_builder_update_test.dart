import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('each setter rebuilds only the builder that issued it', (
    tester,
  ) async {
    var firstBuilds = 0;
    var secondBuilds = 0;
    late StateSetter first;
    late StateSetter second;
    await tester.pumpWidget(
      Column(
        children: <Widget>[
          StatefulBuilder(
            builder: (context, setter) {
              first = setter;
              firstBuilds++;
              return const SizedBox.square(dimension: 20);
            },
          ),
          StatefulBuilder(
            builder: (context, setter) {
              second = setter;
              secondBuilds++;
              return const SizedBox.square(dimension: 20);
            },
          ),
        ],
      ),
    );
    expect((firstBuilds, secondBuilds), (1, 1));
    first(() {});
    first(() {});
    await tester.pump();
    expect((firstBuilds, secondBuilds), (2, 1));
    second(() {});
    await tester.pump();
    expect((firstBuilds, secondBuilds), (2, 2));
  });

  testWidgets('setter survives the parent update but not its own unmount', (
    tester,
  ) async {
    var count = 0;
    StateSetter? original;
    Widget scene(String? prefix) => Directionality(
      textDirection: TextDirection.ltr,
      child: Column(
        children: <Widget>[
          if (prefix != null)
            StatefulBuilder(
              key: const ValueKey<String>('retained'),
              builder: (context, setter) {
                original ??= setter;
                return Text('$prefix $count');
              },
            ),
        ],
      ),
    );
    await tester.pumpWidget(scene('before'));
    expect(find.text('before 0'), findsOneWidget);
    await tester.pumpWidget(scene('after'));
    expect(find.text('after 0'), findsOneWidget);
    original!(() => count = 1);
    await tester.pump();
    expect(find.text('after 1'), findsOneWidget);
    await tester.pumpWidget(scene(null));
    expect(() => original!(() => count = 2), throwsFlutterError);
    expect(count, 1);
  });
}
