import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class Reader extends StatefulWidget {
  const Reader(this.changes, {super.key});
  final List<bool> changes;

  @override
  State<Reader> createState() => _ReaderState();
}

class _ReaderState extends State<Reader> {
  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    widget.changes.add(Visibility.of(context));
  }

  @override
  Widget build(BuildContext context) {
    return Text(Visibility.of(context) ? 'shown' : 'hidden');
  }
}

void main() {
  testWidgets('visibility observes a hidden ancestor, then stops the walk', (
    tester,
  ) async {
    final changes = <bool>[];
    final reader = Reader(changes);
    Future<void> pump(bool outer, bool inner) => tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Visibility(
          visible: outer,
          maintainState: true,
          child: Center(
            child: Visibility(
              visible: inner,
              maintainState: true,
              child: reader,
            ),
          ),
        ),
      ),
    );

    await pump(true, false);
    expect(changes, [false]);
    await pump(false, false);
    expect(changes, [false], reason: 'outer scope was never read');
    await pump(true, true);
    expect(changes, [false, true]);
    await pump(true, false);
    expect(changes, [false, true, false]);
    await pump(false, false);
    expect(changes, [false, true, false, false], reason: 'outer was observed');
    await pump(false, false);
    expect(changes, [false, true, false, false]);
  });

  testWidgets('a replacement sees the false scope without hiding its paint', (
    tester,
  ) async {
    final changes = <bool>[];
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Visibility(
          visible: false,
          replacement: Reader(changes),
          child: const Text('original'),
        ),
      ),
    );
    expect(changes, [false]);
    expect(find.text('hidden'), findsOneWidget);
    expect(find.text('original', skipOffstage: false), findsNothing);
  });
}
