import 'package:flutter/gestures.dart';
import 'package:flutter_test/flutter_test.dart';

class RecordingMember implements GestureArenaMember {
  RecordingMember(this.name, this.decisions);

  final String name;
  final List<(String, GestureDisposition, int)> decisions;

  @override
  void acceptGesture(int pointer) {
    decisions.add((name, GestureDisposition.accepted, pointer));
  }

  @override
  void rejectGesture(int pointer) {
    decisions.add((name, GestureDisposition.rejected, pointer));
  }
}

void main() {
  test('Late rejection does not rewrite a resolved winner', () {
    const pointer = 4;
    final arena = GestureArenaManager();
    final decisions = <(String, GestureDisposition, int)>[];
    final winner = arena.add(pointer, RecordingMember('winner', decisions));
    final loser = arena.add(pointer, RecordingMember('loser', decisions));
    arena.close(pointer);
    expect(decisions, isEmpty);

    winner.resolve(GestureDisposition.accepted);
    expect(decisions, [
      ('loser', GestureDisposition.rejected, pointer),
      ('winner', GestureDisposition.accepted, pointer),
    ]);

    final resolved = decisions.toList();
    winner.resolve(GestureDisposition.rejected);
    expect(decisions, resolved);
    loser.resolve(GestureDisposition.accepted);
    expect(decisions, resolved);
  });
}
